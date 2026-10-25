import {
    AstUtils, DefaultScopeComputation, DefaultScopeProvider, DocumentState, EMPTY_STREAM, isReference, stream, WorkspaceCache
} from 'langium';
import type { AstNode, AstNodeDescription, LangiumCoreServices, LangiumDocument, ReferenceInfo, Scope, Stream } from 'langium';
import {
    isActionUsage, isActorUsage, isAliasMember, isAttributeUsage, isConnectorEnd, isDefaultUsage, isImportDecl, isModel,
    isOtherUsage, isPartDef, isPartUsage, isPerformUsage, isPortUsage, isSubjectUsage, isUseCaseUsage
} from '../generated/ast.js';
import type { AliasMember, ImportDecl, PartDef } from '../generated/ast.js';
import { effectiveName, qualifiedName } from '../model/graph.js';

/**
 * Exports every named element under its fully qualified name
 * (e.g. `Drone::FlightController`): the top-level packages are where a name
 * lookup ends.
 */
export class SysmlScopeComputation extends DefaultScopeComputation {
    override async collectExportedSymbols(document: LangiumDocument): Promise<AstNodeDescription[]> {
        const exports: AstNodeDescription[] = [];
        for (const node of AstUtils.streamAllContents(document.parseResult.value)) {
            const name = this.nameProvider.getName(node);
            if (name) {
                exports.push(this.descriptions.createDescription(node, qualifiedName(node), document));
            }
        }
        return exports;
    }
}

/**
 * Resolves names as SysML v2 does. A name's first segment is looked up in the
 * namespace it is written in, among its own members and aliases, what a
 * definition inherits through `:>` and what a namespace imports, then in each
 * namespace around it, and last among the top-level elements. Each further
 * `::` segment is a member of the one before, or something it imports
 * publicly. An element answers to its short name too, and an unnamed one to
 * the name it takes on (`effectiveName`).
 *
 * Connector-end chains continue with dots: each segment after the first is a
 * feature of the one before — of its own, of its type, inherited ones
 * included, or of what it subsets or references.
 */
export class SysmlScopeProvider extends DefaultScopeProvider {
    private readonly topLevel: WorkspaceCache<string, Map<string, AstNode>>;
    /**
     * Namespaces whose import is being resolved. A lookup does not consult
     * their imports, which would be resolving the import by itself.
     */
    private readonly resolvingImports = new Set<AstNode>();
    /** Aliases being followed, so one naming itself ends rather than loops. */
    private readonly followingAliases = new Set<AliasMember>();
    /** Features whose features are being gathered, so one subsetting itself ends. */
    private readonly gathering = new Set<AstNode>();

    constructor(services: LangiumCoreServices) {
        super(services);
        this.topLevel = new WorkspaceCache(services.shared, DocumentState.IndexedContent);
    }

    override getScope(context: ReferenceInfo): Scope {
        const container = context.container;
        const index = context.index ?? 0;
        if (isConnectorEnd(container) && context.property === 'segments' && index > 0) {
            const previous = container.segments[index - 1]?.ref;
            return new ResolvingScope(name => {
                const found = this.featureNamed(previous, name);
                return found ? this.descriptions.createDescription(found, name) : undefined;
            });
        }
        const type = this.reflection.getReferenceType(context);
        const importer = isImportDecl(container) ? container.$container : undefined;
        // A reference's text is its names, unquoted, joined by `::`.
        return new ResolvingScope(name => {
            const found = this.withoutImportsOf(importer, () => this.resolve(container, name.split('::'), type));
            return found ? this.descriptions.createDescription(found, name) : undefined;
        });
    }

    private withoutImportsOf<T>(namespace: AstNode | undefined, lookUp: () => T): T {
        if (!namespace || this.resolvingImports.has(namespace)) {
            return lookUp();
        }
        this.resolvingImports.add(namespace);
        try {
            return lookUp();
        } finally {
            this.resolvingImports.delete(namespace);
        }
    }

    private resolve(from: AstNode, segments: string[], type: string | undefined): AstNode | undefined {
        if (segments.length === 0) {
            return undefined;
        }
        const fits = (i: number) => i === segments.length - 1 ? type : undefined;
        let found = this.lookUp(from, segments[0], fits(0));
        for (let i = 1; found && i < segments.length; i++) {
            found = this.visibleMember(found, segments[i], fits(i), new Set());
        }
        return found;
    }

    private lookUp(from: AstNode, name: string, type: string | undefined): AstNode | undefined {
        for (let namespace: AstNode | undefined = from; namespace; namespace = namespace.$container) {
            // A definition's own supertypes are not looked up among what it inherits.
            const found = this.ownedMember(namespace, name, type, namespace !== from, from)
                ?? this.imported(namespace, name, type, false, new Set());
            if (found) {
                return found;
            }
        }
        const topLevel = this.topLevelElements().get(name);
        return topLevel && this.fits(topLevel, type) ? topLevel : undefined;
    }

    /**
     * What a namespace shows outside: its own members and its public imports,
     * or all its imports for an `import all`.
     */
    private visibleMember(
        namespace: AstNode, name: string, type: string | undefined, seen: Set<AstNode>, all = false
    ): AstNode | undefined {
        return this.ownedMember(namespace, name, type) ?? this.imported(namespace, name, type, !all, seen);
    }

    private ownedMember(
        namespace: AstNode, name: string, type: string | undefined, inherited = true, from?: AstNode
    ): AstNode | undefined {
        for (const member of AstUtils.streamContents(namespace)) {
            // An unnamed element takes its name from what it refers to, so it
            // does not answer that reference itself: `perform fly` names fly.
            if (from && (member as { name?: string }).name === undefined && AstUtils.hasContainerOfType(from, n => n === member)) {
                continue;
            }
            if (isAliasMember(member)) {
                const target = answersTo(member, name) ? this.aliased(member) : undefined;
                if (target && this.fits(target, type)) {
                    return target;
                }
            } else if (answersTo(member, name) && this.fits(member, type)) {
                return member;
            }
        }
        if (inherited && isPartDef(namespace)) {
            return inheritedFeatures(namespace).find(feature => answersTo(feature, name) && this.fits(feature, type));
        }
        return undefined;
    }

    private aliased(alias: AliasMember): AstNode | undefined {
        if (this.followingAliases.has(alias)) {
            return undefined;
        }
        this.followingAliases.add(alias);
        try {
            return this.resolve(alias.$container, alias.target.split('::'), undefined);
        } finally {
            this.followingAliases.delete(alias);
        }
    }

    /**
     * What a namespace's imports bring in: `Pkg::Element` that element, under
     * the name the import gives it, `Pkg::*` the members of Pkg, and `::**`
     * everything nested below as well. An import of a package ortho does not
     * have, such as the standard library's, brings in nothing.
     */
    private imported(
        namespace: AstNode, name: string, type: string | undefined, publicOnly: boolean, seen: Set<AstNode>
    ): AstNode | undefined {
        const imports = (namespace as { imports?: ImportDecl[] }).imports;
        if (!imports || seen.has(namespace) || this.resolvingImports.has(namespace)) {
            return undefined;
        }
        seen.add(namespace);
        for (const imp of imports) {
            if (publicOnly && imp.visibility !== 'public') {
                continue;
            }
            const target = imp.target?.ref;
            if (!target) {
                continue;
            }
            // An alias imported by name brings in what it names, under its own name.
            const importedName = imp.target.$refText.split('::').at(-1);
            const found = (imp.wildcard
                ? this.visibleMember(target, name, type, seen, imp.all)
                : (answersTo(target, name) || importedName === name) && this.fits(target, type) ? target : undefined)
                ?? (imp.recursive ? this.nestedMember(target, name, type, seen) : undefined);
            if (found) {
                return found;
            }
        }
        return undefined;
    }

    private nestedMember(namespace: AstNode, name: string, type: string | undefined, seen: Set<AstNode>): AstNode | undefined {
        for (const member of AstUtils.streamContents(namespace)) {
            const found = this.visibleMember(member, name, type, seen) ?? this.nestedMember(member, name, type, seen);
            if (found) {
                return found;
            }
        }
        return undefined;
    }

    /** A feature of a feature, or an alias declared among them. */
    private featureNamed(feature: AstNode | undefined, name: string): AstNode | undefined {
        const features = this.featuresOf(feature);
        const found = features.find(f => answersTo(f, name));
        if (found) {
            return found;
        }
        const type = feature && typeOf(feature);
        const alias = [...aliasesOf(feature), ...aliasesOf(type)].find(a => answersTo(a, name));
        return alias ? this.aliased(alias) : undefined;
    }

    /**
     * A feature's features: its own, those of its type and the type's
     * supertypes, and those of what it subsets or references — a perform
     * `perform action takePhoto references takePicture` has takePicture's.
     */
    private featuresOf(feature: AstNode | undefined): AstNode[] {
        if (!feature || this.gathering.has(feature)) {
            return [];
        }
        this.gathering.add(feature);
        try {
            const type = typeOf(feature);
            const related = feature as { subsets?: string[]; references?: string[] };
            return [
                ...ownFeatures(feature),
                ...(type ? definitionFeatures(type, new Set()) : []),
                ...[...related.subsets ?? [], ...related.references ?? []]
                    .flatMap(chain => this.featuresOf(this.resolveChain(feature, chain))),
                ...(isPerformUsage(feature) ? this.featuresOf(feature.target?.segments.at(-1)?.ref) : [])
            ];
        } finally {
            this.gathering.delete(feature);
        }
    }

    /** A chain written as text, `a.b.c`, resolved from where its feature stands. */
    private resolveChain(feature: AstNode, chain: string): AstNode | undefined {
        const [first, ...rest] = chain.split('.');
        let found = this.resolve(feature.$container ?? feature, first.split('::'), undefined);
        for (const name of rest) {
            found = found && found !== feature ? this.featureNamed(found, name) : undefined;
        }
        return found === feature ? undefined : found;
    }

    private fits(node: AstNode, type: string | undefined): boolean {
        return !type || this.reflection.isInstance(node, type);
    }

    /** The elements outside any package, by name and by short name. */
    private topLevelElements(): Map<string, AstNode> {
        return this.topLevel.get('all', () => {
            const elements = new Map<string, AstNode>();
            for (const node of this.indexManager.allElements().map(d => d.node)) {
                if (node && isModel(node.$container)) {
                    for (const name of [effectiveName(node), (node as { shortName?: string }).shortName]) {
                        if (name && !elements.has(name)) {
                            elements.set(name, node);
                        }
                    }
                }
            }
            return elements;
        });
    }
}

function answersTo(node: AstNode, name: string): boolean {
    return effectiveName(node) === name || (node as { shortName?: string }).shortName === name;
}

/** A scope that resolves each name as it is asked for. */
class ResolvingScope implements Scope {
    constructor(private readonly resolveName: (name: string) => AstNodeDescription | undefined) {}

    getElement(name: string): AstNodeDescription | undefined {
        return this.resolveName(name);
    }

    getElements(name: string): Stream<AstNodeDescription> {
        const found = this.resolveName(name);
        return found ? stream([found]) : EMPTY_STREAM;
    }

    getAllElements(): Stream<AstNodeDescription> {
        return EMPTY_STREAM;
    }
}

/** What a usage is typed by, where that is a reference ortho resolves. */
function typeOf(feature: AstNode): AstNode | undefined {
    const type = (feature as { type?: unknown }).type;
    return isReference(type) ? type.ref : undefined;
}

function ownFeatures(node: AstNode): AstNode[] {
    return ((node as { members?: AstNode[] }).members ?? []).filter(isNamedFeature);
}

function aliasesOf(node: AstNode | undefined): AliasMember[] {
    return (node as { aliases?: AliasMember[] } | undefined)?.aliases ?? [];
}

/** A definition's features, and those of the definitions it specializes. */
function definitionFeatures(def: AstNode, seen: Set<AstNode>): AstNode[] {
    if (seen.has(def)) {
        return [];
    }
    seen.add(def);
    const features = ownFeatures(def);
    for (const supertype of (def as { supertypes?: unknown[] }).supertypes ?? []) {
        if (isReference(supertype) && supertype.ref) {
            features.push(...definitionFeatures(supertype.ref, seen));
        }
    }
    return features;
}

/**
 * Features a definition inherits through `:>` but does not declare itself.
 * Locally declared members shadow inherited ones, so only genuinely inherited
 * names are added.
 */
function inheritedFeatures(def: PartDef): AstNode[] {
    const taken = new Set(ownFeatures(def).map(effectiveName));
    const inherited: AstNode[] = [];
    for (const supertype of def.supertypes) {
        for (const feature of supertype.ref ? definitionFeatures(supertype.ref, new Set()) : []) {
            const name = effectiveName(feature);
            if (name && !taken.has(name)) {
                taken.add(name);
                inherited.push(feature);
            }
        }
    }
    return inherited;
}

function isNamedFeature(node: AstNode): boolean {
    return isPartUsage(node) || isPortUsage(node) || isAttributeUsage(node) || isActionUsage(node) || isUseCaseUsage(node)
        || isActorUsage(node) || isSubjectUsage(node) || isPerformUsage(node) || isOtherUsage(node) || isDefaultUsage(node);
}

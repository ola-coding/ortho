import { AstUtils, DefaultScopeComputation, DefaultScopeProvider, DocumentState, EMPTY_STREAM, stream, WorkspaceCache } from 'langium';
import type { AstNode, AstNodeDescription, LangiumCoreServices, LangiumDocument, ReferenceInfo, Scope, Stream } from 'langium';
import {
    isActionUsage, isAliasMember, isAttributeUsage, isConnectorEnd, isImportDecl, isInterfaceEnd, isModel, isPartDef,
    isPartUsage, isPortUsage, isSubjectUsage, isUseCaseUsage
} from '../generated/ast.js';
import type { AliasMember, Feature, ImportDecl, PartDef, PortDef } from '../generated/ast.js';
import { qualifiedName } from '../model/graph.js';

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
                exports.push(this.descriptions.createDescription(node, qualifiedName(node as AstNode & { name: string }), document));
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
 * publicly. An element answers to its short name as well as its name.
 *
 * Connector-end chains continue with dots: each segment after the first is a
 * feature of the previous segment's type, inherited members included.
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

    constructor(services: LangiumCoreServices) {
        super(services);
        this.topLevel = new WorkspaceCache(services.shared, DocumentState.IndexedContent);
    }

    override getScope(context: ReferenceInfo): Scope {
        const container = context.container;
        const index = context.index ?? 0;
        if (isConnectorEnd(container) && context.property === 'segments' && index > 0) {
            const members = featureMembers(container.segments[index - 1]?.ref);
            return new ResolvingScope(name => {
                const found = members.find(member => answersTo(member, name));
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
            const found = this.ownedMember(namespace, name, type, namespace !== from)
                ?? this.imported(namespace, name, type, false, new Set());
            if (found) {
                return found;
            }
        }
        const topLevel = this.topLevelElements().get(name);
        return topLevel && this.fits(topLevel, type) ? topLevel : undefined;
    }

    /** What a namespace shows outside: its own members and its public imports. */
    private visibleMember(namespace: AstNode, name: string, type: string | undefined, seen: Set<AstNode>): AstNode | undefined {
        return this.ownedMember(namespace, name, type) ?? this.imported(namespace, name, type, true, seen);
    }

    private ownedMember(namespace: AstNode, name: string, type: string | undefined, inherited = true): AstNode | undefined {
        for (const member of AstUtils.streamContents(namespace)) {
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
     * What a namespace's imports bring in: `Pkg::Element` that element,
     * `Pkg::*` the members of Pkg, and `::**` everything nested below as well.
     * An import of a package ortho does not have, such as the standard
     * library's, brings in nothing.
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
            const found = (imp.wildcard
                ? this.visibleMember(target, name, type, seen)
                : answersTo(target, name) && this.fits(target, type) ? target : undefined)
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

    private fits(node: AstNode, type: string | undefined): boolean {
        return !type || this.reflection.isInstance(node, type);
    }

    /** The elements outside any package, by name and by short name. */
    private topLevelElements(): Map<string, AstNode> {
        return this.topLevel.get('all', () => {
            const elements = new Map<string, AstNode>();
            for (const node of this.indexManager.allElements().map(d => d.node)) {
                if (node && isModel(node.$container)) {
                    for (const name of [(node as Named).name, (node as Named).shortName]) {
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

interface Named {
    name?: string;
    shortName?: string;
}

function answersTo(node: AstNode, name: string): boolean {
    return (node as Named).name === name || (node as Named).shortName === name;
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

/**
 * Features a definition inherits through `:>` but does not declare itself.
 * Locally declared members shadow inherited ones, so only genuinely inherited
 * names are added.
 */
function inheritedFeatures(def: PartDef): AstNode[] {
    const taken = new Set(def.members.filter(isNamedFeature).map(member => (member as { name?: string }).name));
    const inherited: AstNode[] = [];
    for (const supertype of def.supertypes) {
        if (!supertype.ref) {
            continue;
        }
        for (const feature of partDefFeatures(supertype.ref, new Set())) {
            const name = (feature as { name?: string }).name;
            if (name && !taken.has(name)) {
                taken.add(name);
                inherited.push(feature);
            }
        }
    }
    return inherited;
}

/** Named features reachable through a feature-chain segment. */
function featureMembers(feature: Feature | undefined): AstNode[] {
    if (isPartUsage(feature)) {
        const members: AstNode[] = feature.members.filter(isNamedFeature);
        if (feature.type?.ref) {
            members.push(...partDefFeatures(feature.type.ref, new Set()));
        }
        return members;
    }
    if (isPortUsage(feature) || isInterfaceEnd(feature)) {
        return feature.type?.ref ? portDefFeatures(feature.type.ref, new Set()) : [];
    }
    if (isSubjectUsage(feature)) {
        return feature.type?.ref ? partDefFeatures(feature.type.ref, new Set()) : [];
    }
    // An action's own nested actions, plus those its definition contributes —
    // how a `perform` chain walks down a function tree.
    if (isActionUsage(feature) || isUseCaseUsage(feature)) {
        const members: AstNode[] = feature.members.filter(isNamedFeature);
        if (isActionUsage(feature) && feature.type?.ref) {
            members.push(...feature.type.ref.members.filter(isNamedFeature));
        }
        return members;
    }
    return [];
}

function partDefFeatures(def: PartDef, seen: Set<PartDef>): AstNode[] {
    if (seen.has(def)) {
        return [];
    }
    seen.add(def);
    const members: AstNode[] = def.members.filter(isNamedFeature);
    for (const supertype of def.supertypes) {
        if (supertype.ref) {
            members.push(...partDefFeatures(supertype.ref, seen));
        }
    }
    return members;
}

function portDefFeatures(def: PortDef, seen: Set<PortDef>): AstNode[] {
    if (seen.has(def)) {
        return [];
    }
    seen.add(def);
    const members: AstNode[] = [...def.members];
    for (const supertype of def.supertypes) {
        if (supertype.ref) {
            members.push(...portDefFeatures(supertype.ref, seen));
        }
    }
    return members;
}

function isNamedFeature(node: AstNode): boolean {
    return isPartUsage(node) || isPortUsage(node) || isAttributeUsage(node)
        || isActionUsage(node) || isUseCaseUsage(node);
}

import { AstUtils, DefaultScopeComputation, DefaultScopeProvider, DocumentState, EMPTY_SCOPE, EMPTY_STREAM, stream, WorkspaceCache } from 'langium';
import type { AstNode, AstNodeDescription, LangiumCoreServices, LangiumDocument, ReferenceInfo, Scope, Stream } from 'langium';
import {
    isActionUsage, isAttributeUsage, isConnectorEnd, isInterfaceEnd, isPackageDecl, isPartDef, isPartUsage,
    isPortUsage, isSubjectUsage, isUseCaseUsage
} from '../generated/ast.js';
import type { Feature, PackageDecl, PartDef, PortDef } from '../generated/ast.js';
import { qualifiedName } from '../model/graph.js';
import { importCandidates } from '../model/packages.js';

/**
 * Exports every named element under its fully qualified name
 * (e.g. `Drone::FlightController`): the top-level packages are where a name
 * lookup ends, and imports find what they import here.
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
 * namespace it is written in, among its own members, what a definition
 * inherits through `:>` and what a package imports, then in each namespace
 * around it, and last among the top-level packages. Each further `::` segment
 * is a member of the one before, or something it imports publicly.
 *
 * Connector-end chains continue with dots: each segment after the first is a
 * feature of the previous segment's type, inherited members included.
 */
export class SysmlScopeProvider extends DefaultScopeProvider {
    private readonly byQualifiedName: WorkspaceCache<string, Map<string, AstNode>>;

    constructor(services: LangiumCoreServices) {
        super(services);
        this.byQualifiedName = new WorkspaceCache(services.shared, DocumentState.IndexedContent);
    }

    override getScope(context: ReferenceInfo): Scope {
        const container = context.container;
        const index = context.index ?? 0;
        if (isConnectorEnd(container) && context.property === 'segments' && index > 0) {
            const members = featureMembers(container.segments[index - 1]?.ref);
            return members.length > 0 ? this.createScopeForNodes(members.values()) : EMPTY_SCOPE;
        }
        const type = this.reflection.getReferenceType(context);
        return new ResolvingScope(name => {
            const found = this.resolve(container, name.split('::'), type);
            return found ? this.descriptions.createDescription(found, name) : undefined;
        });
    }

    private resolve(from: AstNode, segments: string[], type: string): AstNode | undefined {
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
                ?? (isPackageDecl(namespace) ? this.imported(namespace, name, type, false, new Set()) : undefined);
            if (found) {
                return found;
            }
        }
        const topLevel = this.qualifiedNames().get(name);
        return topLevel && this.fits(topLevel, type) ? topLevel : undefined;
    }

    /** What a namespace shows outside: its own members and its public imports. */
    private visibleMember(namespace: AstNode, name: string, type: string | undefined, seen: Set<PackageDecl>): AstNode | undefined {
        return this.ownedMember(namespace, name, type)
            ?? (isPackageDecl(namespace) ? this.imported(namespace, name, type, true, seen) : undefined);
    }

    private ownedMember(namespace: AstNode, name: string, type: string | undefined, inherited = true): AstNode | undefined {
        const members = [
            ...AstUtils.streamContents(namespace),
            ...(inherited && isPartDef(namespace) ? inheritedFeatures(namespace) : [])
        ];
        return members.find(member => this.nameProvider.getName(member) === name && this.fits(member, type));
    }

    private imported(
        pkg: PackageDecl, name: string, type: string | undefined, publicOnly: boolean, seen: Set<PackageDecl>
    ): AstNode | undefined {
        if (seen.has(pkg)) {
            return undefined;
        }
        seen.add(pkg);
        for (const imp of pkg.imports) {
            if (publicOnly && imp.visibility !== 'public') {
                continue;
            }
            // A package ortho does not have, such as the standard library's, imports nothing.
            const target = importCandidates(pkg, imp.path).map(fqn => this.qualifiedNames().get(fqn)).find(Boolean);
            const found = !target ? undefined
                : imp.wildcard ? this.visibleMember(target, name, type, seen)
                : this.nameProvider.getName(target) === name && this.fits(target, type) ? target : undefined;
            if (found) {
                return found;
            }
        }
        return undefined;
    }

    private fits(node: AstNode, type: string | undefined): boolean {
        return !type || this.reflection.isInstance(node, type);
    }

    private qualifiedNames(): Map<string, AstNode> {
        return this.byQualifiedName.get('all', () => new Map(
            this.indexManager.allElements().filter(d => d.node !== undefined).map(d => [d.name, d.node!] as const)
        ));
    }
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

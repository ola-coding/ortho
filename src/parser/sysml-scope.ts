import { AstUtils, DefaultScopeComputation, DefaultScopeProvider, EMPTY_SCOPE, MultiMap } from 'langium';
import type { AstNode, AstNodeDescription, LangiumDocument, PrecomputedScopes, ReferenceInfo, Scope } from 'langium';
import {
    isAttributeUsage, isConnectorEnd, isInterfaceEnd, isPartDef, isPartUsage, isPortUsage, isSubjectUsage
} from '../generated/ast.js';
import type { Feature, PartDef, PortDef } from '../generated/ast.js';
import { qualifiedName } from '../model/graph.js';

/**
 * Exports every named element under its fully qualified name
 * (e.g. `Drone::FlightController`), so cross-package references work without
 * import resolution. Same-package references keep working through the default
 * local scopes, which use simple names.
 */
export class SysmlScopeComputation extends DefaultScopeComputation {
    override async computeExports(document: LangiumDocument): Promise<AstNodeDescription[]> {
        const exports: AstNodeDescription[] = [];
        for (const node of AstUtils.streamAllContents(document.parseResult.value)) {
            const name = this.nameProvider.getName(node);
            if (name) {
                exports.push(this.descriptions.createDescription(node, qualifiedName(node as AstNode & { name: string }), document));
            }
        }
        return exports;
    }

    /**
     * Besides the default simple-name-in-direct-container scopes, every named
     * element is also visible in each enclosing container under its relatively
     * qualified name, so `Physical::fc` resolves from a sibling package.
     */
    override async computeLocalScopes(document: LangiumDocument): Promise<PrecomputedScopes> {
        const scopes = new MultiMap<AstNode, AstNodeDescription>();
        for (const node of AstUtils.streamAllContents(document.parseResult.value)) {
            const name = this.nameProvider.getName(node);
            if (!name || !node.$container) {
                continue;
            }
            scopes.add(node.$container, this.descriptions.createDescription(node, name, document));
            let relativeName = name;
            let container: AstNode = node.$container;
            while (container.$container) {
                const containerName = (container as { name?: string }).name;
                if (!containerName) {
                    break;
                }
                relativeName = `${containerName}::${relativeName}`;
                scopes.add(container.$container, this.descriptions.createDescription(node, relativeName, document));
                container = container.$container;
            }
        }
        return scopes;
    }
}

/**
 * Resolves connector-end feature chains:
 *
 * - segments after the first (`battery.powerOut`) against the members of the
 *   previous segment's type;
 * - the first segment against the features the enclosing definition inherits
 *   through `:>`, so a realization can connect to a port declared on the
 *   definition it specializes.
 */
export class SysmlScopeProvider extends DefaultScopeProvider {
    override getScope(context: ReferenceInfo): Scope {
        const container = context.container;
        if (isConnectorEnd(container) && context.property === 'segments') {
            const index = context.index ?? 0;
            if (index > 0) {
                const previous = container.segments[index - 1]?.ref;
                const members = featureMembers(previous);
                if (members.length === 0) {
                    return EMPTY_SCOPE;
                }
                return this.createScopeForNodes(members.values());
            }
            const inherited = inheritedFeatures(container);
            if (inherited.length > 0) {
                return this.createScopeForNodes(inherited.values(), super.getScope(context));
            }
        }
        return super.getScope(context);
    }
}

/**
 * Features the enclosing definition inherits through `:>` but does not declare
 * itself. Locally declared members shadow inherited ones and already resolve
 * through the default scope, so only genuinely inherited names are added.
 */
function inheritedFeatures(node: AstNode): AstNode[] {
    let container: AstNode | undefined = node.$container;
    while (container && !isPartDef(container)) {
        container = container.$container;
    }
    if (!container) {
        return [];
    }
    const taken = new Set(
        container.members.filter(isNamedFeature).map(member => (member as { name?: string }).name)
    );
    const inherited: AstNode[] = [];
    for (const supertype of container.supertypes) {
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
    return isPartUsage(node) || isPortUsage(node) || isAttributeUsage(node);
}

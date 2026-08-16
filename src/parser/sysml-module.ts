import { inject, createDefaultCoreModule, createDefaultSharedCoreModule } from 'langium';
import type { LangiumCoreServices, LangiumSharedCoreServices, Module, PartialLangiumCoreServices } from 'langium';
import { NodeFileSystem } from 'langium/node';
import { SysmlGeneratedModule, SysmlGeneratedSharedModule } from '../generated/module.js';
import { SysmlScopeComputation, SysmlScopeProvider } from './sysml-scope.js';

export type SysmlServices = LangiumCoreServices;

export const SysmlModule: Module<SysmlServices, PartialLangiumCoreServices> = {
    references: {
        ScopeComputation: (services) => new SysmlScopeComputation(services),
        ScopeProvider: (services) => new SysmlScopeProvider(services)
    }
};

export function createSysmlServices(): {
    shared: LangiumSharedCoreServices;
    Sysml: SysmlServices;
} {
    const shared = inject(
        createDefaultSharedCoreModule(NodeFileSystem),
        SysmlGeneratedSharedModule
    );
    const Sysml = inject(
        createDefaultCoreModule({ shared }),
        SysmlGeneratedModule,
        SysmlModule
    );
    shared.ServiceRegistry.register(Sysml);
    return { shared, Sysml };
}

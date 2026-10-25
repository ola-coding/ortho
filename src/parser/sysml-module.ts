import { inject, createDefaultCoreModule, createDefaultSharedCoreModule } from 'langium';
import type { LangiumCoreServices, LangiumSharedCoreServices, Module, PartialLangiumCoreServices } from 'langium';
import { NodeFileSystem } from 'langium/node';
import { SysmlGeneratedModule, SysmlGeneratedSharedModule } from '../generated/module.js';
import { SysmlDocumentValidator } from './sysml-document-validator.js';
import { SysmlScopeComputation, SysmlScopeProvider } from './sysml-scope.js';
import { registerValidationChecks } from './sysml-validator.js';
import { SysmlValueConverter } from './sysml-value-converter.js';

export type SysmlServices = LangiumCoreServices;

export const SysmlModule: Module<SysmlServices, PartialLangiumCoreServices> = {
    parser: {
        ValueConverter: () => new SysmlValueConverter()
    },
    references: {
        ScopeComputation: (services) => new SysmlScopeComputation(services),
        ScopeProvider: (services) => new SysmlScopeProvider(services)
    },
    validation: {
        DocumentValidator: (services) => new SysmlDocumentValidator(services)
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
    registerValidationChecks(Sysml);
    return { shared, Sysml };
}

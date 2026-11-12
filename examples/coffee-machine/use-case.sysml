// Use case view: what people ask the machine for, and who asks.
package SystemUseCases {
    doc /* Nothing here says how the machine works. */
    private import StandardViewDefinitions::*;

    // The subject is the machine as a black box: a part def of its own, naming
    // no hardware. physical.sysml opens it up.
    part def CoffeeMachine;
    part def User;

    use case def OperateMachine {
        subject machine : CoffeeMachine;
        actor user : User;
    }

    use case makeCappuccino : OperateMachine {
        doc /* An espresso the user also wants milk on. Both are beverages the user
             * orders, so the include is reuse, not decomposition. */
        include makeEspresso;
    }

    use case makeEspresso : OperateMachine;

    use case descale : OperateMachine {
        doc /* Looking after the machine is the user's job too: a task the machine
             * asks its owner for, not a call-out. */
    }

    view 'Use case view — Coffee Machine' : GeneralView {
        expose SystemUseCases;
    }
}

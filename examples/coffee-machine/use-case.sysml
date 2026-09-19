// Use case view: what people ask the machine for, and who asks.
//
// Nothing here says how the machine works. The subject is untyped on purpose —
// naming a hardware part would be realization detail, and this view has none.
package UseCases {

    part def User;

    use case def OperateMachine {
        subject CoffeeMachine;
        actor user : User;
    }

    // Looking after the machine is the user's job too: descaling is a task
    // the machine asks its owner for, not a call-out.
    use case def ServiceMachine {
        subject CoffeeMachine;
        actor user : User;
    }

    use case makeEspresso : OperateMachine;

    // A cappuccino is an espresso the user also wants milk on; both are
    // beverages a customer orders, so the include is usage, not decomposition.
    use case makeCappuccino : OperateMachine {
        include use case makeEspresso;
    }

    use case descale : ServiceMachine;
}

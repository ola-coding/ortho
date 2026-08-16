// Scenarios: the three beverages a user can ask for.
// The subject is the system-level part, not a domain component.
package UseCases {

    import System::*;

    part def User;

    use case def OperateMachine {
        subject machine : System::CoffeeMachine;
        actor user : User;
    }

    use case makeCoffee : OperateMachine;

    use case makeEspresso : OperateMachine;

    use case makeCappuccino : OperateMachine {
        include use case makeEspresso;
    }
}

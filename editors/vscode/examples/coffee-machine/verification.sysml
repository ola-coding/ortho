// Verification: how each requirement is shown to be met, and by which method.
package SystemVerification {
    doc /* Each verification case names the requirements it shows are met with
         * `verify`, in its objective, and its method with `VerificationMethod`:
         * inspection, analysis, demonstration or test. A test of a requirement
         * does not verify those nested in it, so each is named on its own. */
    private import VerificationCases::*;
    private import SystemRequirements::*;
    private import Hardware::CoffeeMachine;

    verification def 'Espresso timing test' {
        doc /* From standby at 20 °C, time ten espressos from the choice to the last
             * drop, and the water's heating from 20 °C to 92 °C. */
        @VerificationMethod { kind = VerificationMethodKind::test; }
        subject machine : CoffeeMachine;
        objective {
            verify 'Espresso time';
            verify 'Espresso time'.'Heat-up time';
        }
    }

    verification def 'Brew test' {
        doc /* Log the water's temperature and the pressure at the coffee bed
             * through ten shots. */
        @VerificationMethod { kind = VerificationMethodKind::test; }
        subject machine : CoffeeMachine;
        objective {
            verify 'Espresso time'.'Brew temperature';
            verify 'Espresso time'.'Brew pressure';
        }
    }

    verification def 'Grind and dose test' {
        doc /* Weigh the ground coffee of shots at the smallest and largest dose,
             * and check that the grinder runs before each shot. */
        @VerificationMethod { kind = VerificationMethodKind::test; }
        subject machine : CoffeeMachine;
        objective {
            verify 'Freshly ground coffee';
            verify 'Dose range';
            verify 'Part requirements'::'Grind time';
        }
    }

    verification def 'Milk and menu demonstration' {
        doc /* Make an espresso and a cappuccino from the touch display, with milk
             * drawn from a carton. */
        @VerificationMethod { kind = VerificationMethodKind::demo; }
        subject machine : CoffeeMachine;
        objective {
            verify 'Milk from the carton';
            verify 'Choice of beverage';
        }
    }

    verification def 'Capacity inspection' {
        doc /* Measure the water tank, the bean hopper and the machine's width. */
        @VerificationMethod { kind = VerificationMethodKind::inspect; }
        subject machine : CoffeeMachine;
        objective {
            verify 'Water capacity';
            verify 'Bean capacity';
            verify 'Machine width';
        }
    }

    verification def 'Safety test' {
        doc /* Try to open the brew group during a shot, and run the heater with
             * the water tank removed. */
        @VerificationMethod { kind = VerificationMethodKind::test; }
        subject machine : CoffeeMachine;
        objective {
            verify 'Safety';
            verify 'Safety'.'Brew group interlock';
            verify 'Safety'.'Dry-run cut-off';
        }
    }

    verification def 'Power analysis' {
        doc /* Add up the rated loads, then measure the draw while heating and
             * frothing at once. */
        @VerificationMethod { kind = (VerificationMethodKind::analyze, VerificationMethodKind::test); }
        subject machine : CoffeeMachine;
        objective {
            verify 'Mains power';
            verify 'Part requirements'::'Heater power';
            verify 'Part requirements'::'Pump power';
        }
    }
}

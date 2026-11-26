// Requirements: what the machine shall do, each stated once, with an ID.
package SystemRequirements {
    doc /* Stated of the machine as a black box, like the use cases: no part or
         * program is named here. Each part or module that meets a requirement
         * says so where it is declared, with `satisfy`, beside its `perform`. */
    private import ModelingMetadata::*;
    private import StandardViewDefinitions::*;

    // A requirement's ID is its short name, and its statement its `doc`. The
    // requirements nested in one are its subrequirements: satisfying it
    // satisfies them too. An ID says where its requirement sits: REQ-1.2 is
    // the second in REQ-1.

    // --- making a beverage -------------------------------------------------
    requirement <'REQ-1'> 'Espresso time' {
        doc /* While the machine is in standby, when the user chooses an espresso,
             * the machine shall deliver it within 60 s. */
        @Rationale { text = "A home machine is used in the rush before work; a minute is as long as its users will wait."; }

        requirement <'REQ-1.1'> 'Heat-up time' {
            doc /* The machine shall heat its brewing water from 20 °C to 92 °C within 30 s. */
            @StatusInfo { status = StatusKind::tbr; }
        }
        requirement <'REQ-1.2'> 'Brew temperature' {
            doc /* While it extracts a shot, the machine shall hold the brewing
                 * water within 2 °C of 92 °C. */
            @Rationale { text = "Below about 90 °C an espresso tastes sour, and above 96 °C bitter."; }
        }
        requirement <'REQ-1.3'> 'Brew pressure' {
            doc /* While it extracts a shot, the machine shall hold the pressure at
                 * the coffee bed at 9 bar, plus or minus 1 bar. */
            @Rationale { text = "Espresso is extracted at about 9 bar: much less leaves it thin, much more makes it bitter."; }
        }
    }

    requirement <'REQ-2'> 'Freshly ground coffee' {
        doc /* When it brews a shot, the machine shall first grind whole beans for
             * it. */
        @Rationale { text = "Ground coffee loses most of its aroma within a quarter of an hour."; }
    }

    requirement <'REQ-3'> 'Dose range' {
        doc /* The machine shall dose between 7 g and 16 g of ground coffee for a
             * shot, as the recipe sets. */
    }

    requirement <'REQ-4'> 'Milk from the carton' {
        doc /* The machine shall froth milk drawn straight from the user's own
             * carton. */
        @Rationale { text = "A milk container has to be emptied, cleaned and kept cold every day; a tube in the carton needs only a rinse."; }
    }

    // --- filling and choosing ----------------------------------------------
    requirement <'REQ-5'> 'Water capacity' {
        doc /* The machine shall hold at least 1.5 L of water. */
        @Rationale { text = "About a day's coffee for a household of four, so the tank needs filling once a day at most."; }
    }

    requirement <'REQ-6'> 'Bean capacity' {
        doc /* The machine shall hold at least 250 g of coffee beans. */
        @Rationale { text = "The smallest common bag of beans then empties into it in one go."; }
    }

    requirement <'REQ-7'> 'Choice of beverage' {
        doc /* The machine shall let the user choose an espresso or a cappuccino on a
             * touch display at least 100 mm across. */
        @StatusInfo { status = StatusKind::tbr; }
    }

    // --- safety ------------------------------------------------------------
    requirement <'REQ-8'> 'Safety' {
        doc /* The machine shall protect its user, and itself, from hot water,
             * steam and overheating. */

        requirement <'REQ-8.1'> 'Brew group interlock' {
            doc /* While the brew group is under pressure, the machine shall keep it
                 * shut. */
            @Rationale { text = "Opened under pressure, it would spray scalding water and grounds."; }
        }
        requirement <'REQ-8.2'> 'Dry-run cut-off' {
            doc /* If no water flows through the heater, then the machine shall
                 * switch the heater off within 2 s. */
            @Rationale { text = "Heated dry, a thermoblock overheats within seconds and can damage the machine."; }
        }
    }

    // --- in the kitchen ----------------------------------------------------
    requirement <'REQ-9'> 'Mains power' {
        doc /* The machine shall run from a 230 V, 50 Hz household socket, drawing
             * at most 1.5 kW. */
        @Rationale { text = "With a 2 kW kettle beside it, both stay within a 16 A kitchen circuit."; }
    }

    requirement <'REQ-10'> 'Machine width' {
        doc /* The machine shall be at most 300 mm wide. */
        @Rationale { text = "It then fits the space of the narrowest standard kitchen cabinet."; }
    }

    view 'Requirements — Coffee Machine' : GridView {
        doc /* What the coffee machine shall do for a household making espresso and
             * cappuccino at home. */
        expose SystemRequirements;
    }
}

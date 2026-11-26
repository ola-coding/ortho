// Verification: how each requirement is shown to be met, and by which method.
package SystemVerification {
    doc /* Each verification case names the requirements it shows are met with
         * `verify`, in its objective, and its method with `VerificationMethod`:
         * inspection, analysis, demonstration or test. A test of a requirement
         * does not verify those nested in it, so each is named on its own. */
    private import VerificationCases::*;
    private import SystemRequirements::*;
    private import SystemUseCases::WebShop;

    verification def 'Search test' {
        doc /* Load 10 000 products, then time searches for words in their names and
             * descriptions. */
        @VerificationMethod { kind = VerificationMethodKind::test; }
        subject shop : WebShop;
        objective {
            verify 'Product search';
            verify 'Catalogue size';
        }
    }

    verification def 'Checkout test' {
        doc /* Fill a cart as a guest, check out with a test card, and time the
             * confirmation. */
        @VerificationMethod { kind = VerificationMethodKind::test; }
        subject shop : WebShop;
        objective {
            verify 'Guest cart';
            verify 'Paid orders';
            verify 'Paid orders'.'Confirmation time';
        }
    }

    verification def 'Card data inspection' {
        doc /* Check in the code and the logs that no card details reach the shop's
             * own functions or tables. */
        @VerificationMethod { kind = VerificationMethodKind::inspect; }
        subject shop : WebShop;
        objective {
            verify 'Paid orders'.'Card details';
        }
    }

    verification def 'Fulfilment demonstration' {
        doc /* Place an order, pick and pack it as the clerk, and follow it to the
             * carrier, with its status and emails, as the customer. */
        @VerificationMethod { kind = VerificationMethodKind::demo; }
        subject shop : WebShop;
        objective {
            verify 'Order fulfilment';
            verify 'Order fulfilment'.'Stock reservation';
            verify 'Order fulfilment'.'Pick list';
            verify 'Order fulfilment'.'Shipment booking';
            verify 'Order status';
            verify 'Order emails';
        }
    }

    verification def 'Availability analysis' {
        doc /* Combine the availability AWS commits to for each service the shop
             * runs on. */
        @VerificationMethod { kind = VerificationMethodKind::analyze; }
        subject shop : WebShop;
        objective {
            verify 'Availability';
        }
    }

    verification def 'Peak load test' {
        doc /* Send orders at ten times their usual rate for an hour, and count the
             * orders recorded. */
        @VerificationMethod { kind = VerificationMethodKind::test; }
        subject shop : WebShop;
        objective {
            verify 'Peak load';
        }
    }
}

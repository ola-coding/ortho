// Requirements: what the shop shall do, each stated once, with an ID.
package SystemRequirements {
    doc /* Stated of the shop as a black box, like the use cases: no service,
         * function or table is named here. Each module that meets a requirement
         * says so where it is declared, with `satisfy`, beside its `perform`. */
    private import ModelingMetadata::*;
    private import StandardViewDefinitions::*;

    // A requirement's ID is its short name, and its statement its `doc`. The
    // requirements nested in one are its subrequirements: satisfying it
    // satisfies them too.

    // --- finding a product -------------------------------------------------
    requirement <'REQ-001'> productSearch {
        doc /* The shop shall find products by any word of their name or
             * description, and show the first results within 1 s. */
        @Rationale { text = "Shoppers who wait for results leave for another shop."; }
    }

    requirement <'REQ-002'> catalogueSize {
        doc /* The shop shall hold at least 10 000 products, each with a name, a
             * description, a price and a picture. */
    }

    // --- ordering and paying -----------------------------------------------
    requirement <'REQ-003'> guestCart {
        doc /* The shop shall keep a customer's cart, with no account or sign-in,
             * until the customer orders. */
        @Rationale { text = "Asked to register before they buy, many shoppers do not."; }
    }

    requirement <'REQ-004'> paidOrders {
        doc /* The shop shall record an order only once its payment is authorised. */
        @Rationale { text = "No stock is then reserved, and nothing shipped, for an order no one paid for."; }

        requirement <'REQ-005'> noCardData {
            doc /* The shop shall pass card details straight to the payment provider,
                 * and store none of them. */
            @Rationale { text = "A shop that never stores card data falls under far fewer PCI DSS requirements."; }
        }
        requirement <'REQ-006'> quickConfirmation {
            doc /* The shop shall confirm a paid order to the customer within 5 s of
                 * checkout. */
        }
    }

    // --- getting it to the customer ----------------------------------------
    requirement <'REQ-007'> orderFulfilment {
        doc /* The shop shall see every paid order through from its stock to the
             * carrier. */

        requirement <'REQ-008'> stockReservation {
            doc /* The shop shall reserve an order's stock as soon as the order is
                 * placed. */
            @Rationale { text = "The last item of a product is then never sold twice."; }
        }
        requirement <'REQ-009'> pickList {
            doc /* The shop shall list the orders for the clerk to pick and pack,
                 * oldest first. */
        }
        requirement <'REQ-010'> shipmentBooking {
            doc /* The shop shall book a collection with the carrier for every packed
                 * order. */
        }
    }

    requirement <'REQ-011'> orderStatus {
        doc /* The shop shall show customers the status of each of their orders. */
    }

    requirement <'REQ-012'> orderEmails {
        doc /* The shop shall email the customer when an order is placed, and again
             * when it ships. */
        @Rationale { text = "Customers who know where their order is do not write to ask."; }
    }

    // --- staying open ------------------------------------------------------
    requirement <'REQ-013'> availability {
        doc /* The shop shall be open for orders 99.9 % of every month. */
        @Rationale { text = "Every hour the shop is down loses that hour's orders."; }
    }

    requirement <'REQ-014'> peakLoad {
        doc /* The shop shall take ten times its usual number of orders at a peak,
             * with no one changing how it is set up. */
        @Rationale { text = "A sale, or a mention in the press, brings peaks no one schedules."; }
    }

    view 'Requirements — Web Shop' : GridView {
        doc /* What the web shop shall do for its customers and its staff. */
        expose SystemRequirements;
    }
}

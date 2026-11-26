// Requirements: what the shop shall do, each stated once, with an ID.
package SystemRequirements {
    doc /* Stated of the shop as a black box, like the use cases: no service,
         * function or table is named here. Each module that meets a requirement
         * says so where it is declared, with `satisfy`, beside its `perform`. */
    private import ModelingMetadata::*;
    private import StandardViewDefinitions::*;

    // A requirement's ID is its short name, and its statement its `doc`. The
    // requirements nested in one are its subrequirements: satisfying it
    // satisfies them too. An ID says where its requirement sits: REQ-1.2 is
    // the second in REQ-1.

    // --- finding a product -------------------------------------------------
    requirement <'REQ-1'> 'Product search' {
        doc /* When a customer searches for a word, the shop shall show the products
             * with that word in their name or description, the first within 1 s. */
        @Rationale { text = "Shoppers who wait for results leave for another shop."; }
    }

    requirement <'REQ-2'> 'Catalogue size' {
        doc /* The shop shall hold at least 10 000 products, each with a name, a
             * description, a price and a picture. */
        @StatusInfo { status = StatusKind::tbr; }
    }

    // --- ordering and paying -----------------------------------------------
    requirement <'REQ-3'> 'Guest cart' {
        doc /* The shop shall keep a customer's cart until the customer orders,
             * signed in or not. */
        @Rationale { text = "Asked to register before they buy, many shoppers do not."; }
    }

    requirement <'REQ-4'> 'Paid orders' {
        doc /* When a customer checks out, the shop shall record the order once its
             * payment is authorised. */
        @Rationale { text = "No stock is then reserved, and nothing shipped, for an order no one paid for."; }

        requirement <'REQ-4.1'> 'Card details' {
            doc /* When a customer pays, the shop shall pass the card details
                 * straight to the payment provider. */
            @Rationale { text = "A shop that never stores card data falls under far fewer PCI DSS requirements."; }
        }
        requirement <'REQ-4.2'> 'Confirmation time' {
            doc /* When a customer checks out, the shop shall confirm the paid order
                 * within 5 s. */
            @StatusInfo { status = StatusKind::tbr; }
        }
    }

    // --- getting it to the customer ----------------------------------------
    requirement <'REQ-5'> 'Order fulfilment' {
        doc /* The shop shall see every paid order through from its stock to the
             * carrier. */

        requirement <'REQ-5.1'> 'Stock reservation' {
            doc /* When an order is placed, the shop shall reserve its stock. */
            @Rationale { text = "The last item of a product is then never sold twice."; }
        }
        requirement <'REQ-5.2'> 'Pick list' {
            doc /* The shop shall list the orders for the clerk to pick and pack,
                 * oldest first. */
        }
        requirement <'REQ-5.3'> 'Shipment booking' {
            doc /* When an order is packed, the shop shall book its collection with
                 * the carrier. */
        }
    }

    requirement <'REQ-6'> 'Order status' {
        doc /* The shop shall show customers the status of each of their orders. */
    }

    requirement <'REQ-7'> 'Order emails' {
        doc /* When an order is placed or ships, the shop shall email the customer.
             * */
        @Rationale { text = "Customers who know where their order is do not write to ask."; }
    }

    // --- staying open ------------------------------------------------------
    requirement <'REQ-8'> 'Availability' {
        doc /* The shop shall be open for orders 99.9 % of every month. */
        @Rationale { text = "Every hour the shop is down loses that hour's orders."; }
    }

    requirement <'REQ-9'> 'Peak load' {
        doc /* While orders arrive at up to ten times their usual rate, the shop
             * shall take them all, as it is set up. */
        @Rationale { text = "A sale, or a mention in the press, brings peaks no one schedules."; }
    }

    view 'Requirements — Web Shop' : GridView {
        doc /* What the web shop shall do for its customers and its staff. */
        expose SystemRequirements;
    }
}

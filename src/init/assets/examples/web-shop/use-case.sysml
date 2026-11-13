// Use case view: what customers and the shop's staff ask the shop for.
package SystemUseCases {
    doc /* Nothing here says how the shop works, or that it runs in a cloud. */
    private import StandardViewDefinitions::*;

    // The subject is the shop as a black box: a part def of its own, naming
    // no service. implementation.sysml opens it up.
    part def WebShop;
    part def Customer;
    part def Clerk;
    part def PaymentProvider;
    part def Carrier;

    use case def Shop {
        subject shop : WebShop;
        actor customer : Customer;
    }

    use case def RunShop {
        subject shop : WebShop;
        actor clerk : Clerk;
    }

    use case findProduct : Shop;

    use case placeOrder : Shop {
        doc /* Every order is paid for as it is placed. */
        include payOrder;
    }

    use case payOrder : Shop {
        doc /* A use case of its own, since the payment provider takes part in
             * paying, not in the rest of the order. */
        subject shop;
        actor paymentProvider : PaymentProvider;
    }

    use case trackOrder : Shop;

    use case fulfilOrder : RunShop {
        doc /* The clerk picks and packs; the carrier collects and delivers. */
        subject shop;
        actor carrier : Carrier;
    }

    view 'Use case view — Web Shop' : GeneralView {
        expose SystemUseCases;
    }
}

// Process view: one scenario, a customer placing an order.
package PlaceOrderScenario {
    doc /* The customer checks out, the order is paid for with the payment
         * provider and recorded, and once it is, fulfilment hears of it.
         * A return and refund would be a scenario of its own. */

    private import Apis::*;
    private import StandardViewDefinitions::*;

    // The lifelines are the actors of use-case.sysml and the modules of
    // implementation.sysml; each message is an item one of their APIs carries.
    occurrence def PlaceOrder {
        part customer : SystemUseCases::Customer;
        part storefront : Storefront::StorefrontApp;
        part orders : Ordering::OrderFunction;
        part payments : Ordering::PaymentFunction;
        part provider : SystemUseCases::PaymentProvider;
        part fulfilment : Fulfilment::FulfilmentFunction;

        // Ordering and paying, while the customer waits.
        message checkout from customer to storefront;
        then message placeOrder of Cart from storefront to orders;
        then message authorise of PaymentRequest from orders to payments;
        then message charge of PaymentRequest from payments to provider;
        then message authorised of PaymentResult from provider to payments;
        then message paid of PaymentResult from payments to orders;
        then message recordOrder from orders to orders;
        then message confirmed of Order from orders to storefront;
        then message showConfirmation from storefront to customer;

        // After the customer has gone: an event on EventBridge. The notifier
        // subscribes to it too and mails the customer, left out here.
        then message orderPlaced of OrderPlaced from orders to fulfilment;
        then message reserveStock from fulfilment to fulfilment;
    }

    view 'Process view — Web Shop' : SequenceView {
        expose PlaceOrder;
    }
}

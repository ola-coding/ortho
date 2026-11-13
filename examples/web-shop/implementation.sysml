// Implementation view: the software, as modules and their dependencies.
//
// Serverless on AWS. The storefront is a single-page app, served from S3
// through CloudFront, calling the back end through API Gateway. Each part of
// the back end is a few Lambda functions with a DynamoDB table of their own,
// and they tell each other what happened through EventBridge rather than call
// each other. There is no hardware of the shop's own, so there is no physical
// or deployment view: AWS decides which computer runs what.

package Aws {
    doc /* The vendor's services the modules are built on. ortho knows the names
         * in OMG's library, but no vendor's, so they are declared here. */
    part def S3;
    part def Lambda;
    part def DynamoDB;
}

package Apis {
    doc /* What the modules offer each other, and what crosses. A module that
         * offers an API declares a port of that type; one that needs it
         * declares the same type conjugated, `~OrderApi`. Events are published
         * on EventBridge, so their port defs are topics, with a publisher at
         * one end and subscribers at the other. */
    private import ScalarValues::*;

    // What crosses. process.sysml sends the same items.
    item def Query {
        attribute text : String;
    }
    item def ProductList;
    item def Cart;
    item def Order;
    item def PaymentRequest {
        attribute amount : Real;
        attribute currency : String;
    }
    item def PaymentResult {
        attribute authorised : Boolean;
    }
    item def OrderPlaced;
    item def OrderShipped;

    // --- called, through API Gateway --------------------------------
    port def CatalogueApi {
        in item query : Query;
        out item products : ProductList;
    }

    port def OrderApi {
        in item cart : Cart;
        out item order : Order;
    }

    port def PaymentApi {
        in item request : PaymentRequest;
        out item result : PaymentResult;
    }

    // --- the payment provider's own API, called over the internet --
    port def ProviderApi {
        in item request : PaymentRequest;
        out item result : PaymentResult;
    }

    // --- published and subscribed, on EventBridge --------------------
    port def OrderEvents {
        out item placed : OrderPlaced;
    }

    port def ShipmentEvents {
        out item shipped : OrderShipped;
    }
}

package Storefront {
    doc /* What the customer's browser runs. */
    private import Functions::*;
    private import Aws::*;

    part def StorefrontApp :> S3 {
        doc /* Keeps the cart in the browser until the customer orders. */
        perform sale.takeOrders.keepCart;
        perform sale.informCustomers.showOrderStatus;

        port catalogue : ~Apis::CatalogueApi;
        port orders : ~Apis::OrderApi;
    }
}

package Catalogue {
    doc /* The products, and finding them. */
    private import Functions::*;
    private import Aws::*;

    part def CatalogueFunction :> Lambda {
        perform sale.presentCatalogue;

        port catalogue : Apis::CatalogueApi;
    }

    part def ProductTable :> DynamoDB;
}

package Ordering {
    doc /* Taking an order and its payment. Says what happened, and leaves the
         * rest to whoever subscribes. */
    private import Functions::*;
    private import Aws::*;

    part def OrderFunction :> Lambda {
        perform sale.takeOrders.recordOrder;

        port orders : Apis::OrderApi;
        port payment : ~Apis::PaymentApi;
        port events : Apis::OrderEvents;
    }

    part def PaymentFunction :> Lambda {
        doc /* Hands the card to the payment provider; card numbers never reach
             * the shop's own tables. */
        perform sale.takeOrders.takePayment;

        port payment : Apis::PaymentApi;
        port provider : ~Apis::ProviderApi;
    }

    part def OrderTable :> DynamoDB;
}

package Fulfilment {
    doc /* Getting the goods to the customer, and saying so. */
    private import Functions::*;
    private import Aws::*;

    part def FulfilmentFunction :> Lambda {
        perform sale.fulfilOrders;

        port orders : ~Apis::OrderEvents;
        port shipments : Apis::ShipmentEvents;
    }

    part def NotifyFunction :> Lambda {
        doc /* Mails the customer through Amazon SES. */
        perform sale.informCustomers.notifyCustomer;

        port orders : ~Apis::OrderEvents;
        port shipments : ~Apis::ShipmentEvents;
    }
}

view 'Implementation view — Web Shop' : StandardViewDefinitions::GeneralView {
    expose Storefront;
    expose Catalogue;
    expose Ordering;
    expose Fulfilment;
    expose Aws;
}

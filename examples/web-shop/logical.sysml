// Logical view: what the shop does, independent of what does it.
package Functions {
    doc /* A capability tree. Every function appears exactly once and has exactly
         * one parent. No service, function or table is named here. */
    private import StandardViewDefinitions::*;
    private import Views::*;
    private import ModelingMetadata::*;

    // Branches in the order an order meets them: the customer finds a product,
    // orders it, it is shipped, and the customer hears how it goes.
    action def SellOnline {

        action presentCatalogue {
            action listProducts;
            action searchProducts;
        }

        action takeOrders {
            action keepCart;
            action takePayment;
            action recordOrder;
        }

        // Picking and packing are the clerk's, and delivering the carrier's:
        // the shop only tells them what to do.
        action fulfilOrders {
            action reserveStock;
            action listForPicking;
            action bookShipment;
        }

        action informCustomers {
            action showOrderStatus;
            action notifyCustomer;
        }
    }

    // One performance of the tree. A `perform` chain has to start at a usage,
    // so the modules that realize a function point into this one.
    action sale : SellOnline;

    // What customers and staff ask for, in more detail.
    #refinement dependency SellOnline to SystemUseCases::findProduct;
    #refinement dependency SellOnline to SystemUseCases::placeOrder;
    #refinement dependency SellOnline to SystemUseCases::payOrder;
    #refinement dependency SellOnline to SystemUseCases::trackOrder;
    #refinement dependency SellOnline to SystemUseCases::fulfilOrder;

    view 'Logical view — Web Shop' : GeneralView {
        expose SellOnline;
        expose SystemUseCases::findProduct;
        expose SystemUseCases::placeOrder;
        expose SystemUseCases::payOrder;
        expose SystemUseCases::trackOrder;
        expose SystemUseCases::fulfilOrder;
        render asTreeDiagram;
    }
}

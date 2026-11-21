# Requirements — Web Shop

<sub>Rendered by ortho from [requirements.sysml](../requirements.sysml). Edit the model, not this file.</sub>

What the web shop shall do for its customers and its staff.

14 requirements, all satisfied.

## Product search

| ID | Statement | Rationale |
|:---|:---|:---|
| REQ-1 | The shop shall find products by any word of their name or description, and show the first results within 1 s. | Shoppers who wait for results leave for another shop. |

## Catalogue size

| ID | Statement |
|:---|:---|
| REQ-2 | The shop shall hold at least 10 000 products, each with a name, a description, a price and a picture. |

## Guest cart

| ID | Statement | Rationale |
|:---|:---|:---|
| REQ-3 | The shop shall keep a customer's cart, with no account or sign-in, until the customer orders. | Asked to register before they buy, many shoppers do not. |

## Paid orders

| ID | Statement | Rationale |
|:---|:---|:---|
| REQ-4 | The shop shall record an order only once its payment is authorised. | No stock is then reserved, and nothing shipped, for an order no one paid for. |

### No stored card data

| ID | Statement | Rationale |
|:---|:---|:---|
| REQ-4.1 | The shop shall pass card details straight to the payment provider, and store none of them. | A shop that never stores card data falls under far fewer PCI DSS requirements. |

### Confirmation time

| ID | Statement |
|:---|:---|
| REQ-4.2 | The shop shall confirm a paid order to the customer within 5 s of checkout. |

## Order fulfilment

| ID | Statement |
|:---|:---|
| REQ-5 | The shop shall see every paid order through from its stock to the carrier. |

### Stock reservation

| ID | Statement | Rationale |
|:---|:---|:---|
| REQ-5.1 | The shop shall reserve an order's stock as soon as the order is placed. | The last item of a product is then never sold twice. |

### Pick list

| ID | Statement |
|:---|:---|
| REQ-5.2 | The shop shall list the orders for the clerk to pick and pack, oldest first. |

### Shipment booking

| ID | Statement |
|:---|:---|
| REQ-5.3 | The shop shall book a collection with the carrier for every packed order. |

## Order status

| ID | Statement |
|:---|:---|
| REQ-6 | The shop shall show customers the status of each of their orders. |

## Order emails

| ID | Statement | Rationale |
|:---|:---|:---|
| REQ-7 | The shop shall email the customer when an order is placed, and again when it ships. | Customers who know where their order is do not write to ask. |

## Availability

| ID | Statement | Rationale |
|:---|:---|:---|
| REQ-8 | The shop shall be open for orders 99.9 % of every month. | Every hour the shop is down loses that hour's orders. |

## Peak load

| ID | Statement | Rationale |
|:---|:---|:---|
| REQ-9 | The shop shall take ten times its usual number of orders at a peak, with no one changing how it is set up. | A sale, or a mention in the press, brings peaks no one schedules. |

## Traceability

| ID | Requirement | Satisfied by |
|:---|:---|:---|
| [REQ-1](#product-search) | Product search | [CatalogueFunction](../implementation.sysml#L111) |
| [REQ-2](#catalogue-size) | Catalogue size | [ProductTable](../implementation.sysml#L118) |
| [REQ-3](#guest-cart) | Guest cart | [StorefrontApp](../implementation.sysml#L93) |
| [REQ-4](#paid-orders) | Paid orders | [OrderFunction](../implementation.sysml#L130) |
| [REQ-4.1](#no-stored-card-data) | &emsp;No stored card data | [PaymentFunction](../implementation.sysml#L140) |
| [REQ-4.2](#confirmation-time) | &emsp;Confirmation time | [OrderFunction](../implementation.sysml#L130) |
| [REQ-5](#order-fulfilment) | Order fulfilment | [FulfilmentFunction](../implementation.sysml#L159) |
| [REQ-5.1](#stock-reservation) | &emsp;Stock reservation | through [REQ-5](#order-fulfilment) |
| [REQ-5.2](#pick-list) | &emsp;Pick list | through [REQ-5](#order-fulfilment) |
| [REQ-5.3](#shipment-booking) | &emsp;Shipment booking | through [REQ-5](#order-fulfilment) |
| [REQ-6](#order-status) | Order status | [StorefrontApp](../implementation.sysml#L93) |
| [REQ-7](#order-emails) | Order emails | [NotifyFunction](../implementation.sysml#L167) |
| [REQ-8](#availability) | Availability | [S3](../implementation.sysml#L17), [Lambda](../implementation.sysml#L20), [DynamoDB](../implementation.sysml#L24) |
| [REQ-9](#peak-load) | Peak load | [Lambda](../implementation.sysml#L20), [DynamoDB](../implementation.sysml#L24) |

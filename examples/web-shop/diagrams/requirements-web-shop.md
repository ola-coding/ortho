# Requirements — Web Shop

<sub>Rendered by ortho from [requirements.sysml](../requirements.sysml). Edit the model, not this file.</sub>

What the web shop shall do for its customers and its staff.

14 requirements, all satisfied.

## productSearch

| ID | Statement | Rationale |
|:---|:---|:---|
| REQ-001 | The shop shall find products by any word of their name or description, and show the first results within 1 s. | Shoppers who wait for results leave for another shop. |

## catalogueSize

| ID | Statement |
|:---|:---|
| REQ-002 | The shop shall hold at least 10 000 products, each with a name, a description, a price and a picture. |

## guestCart

| ID | Statement | Rationale |
|:---|:---|:---|
| REQ-003 | The shop shall keep a customer's cart, with no account or sign-in, until the customer orders. | Asked to register before they buy, many shoppers do not. |

## paidOrders

| ID | Statement | Rationale |
|:---|:---|:---|
| REQ-004 | The shop shall record an order only once its payment is authorised. | No stock is then reserved, and nothing shipped, for an order no one paid for. |

### noCardData

| ID | Statement | Rationale |
|:---|:---|:---|
| REQ-005 | The shop shall pass card details straight to the payment provider, and store none of them. | A shop that never stores card data falls under far fewer PCI DSS requirements. |

### quickConfirmation

| ID | Statement |
|:---|:---|
| REQ-006 | The shop shall confirm a paid order to the customer within 5 s of checkout. |

## orderFulfilment

| ID | Statement |
|:---|:---|
| REQ-007 | The shop shall see every paid order through from its stock to the carrier. |

### stockReservation

| ID | Statement | Rationale |
|:---|:---|:---|
| REQ-008 | The shop shall reserve an order's stock as soon as the order is placed. | The last item of a product is then never sold twice. |

### pickList

| ID | Statement |
|:---|:---|
| REQ-009 | The shop shall list the orders for the clerk to pick and pack, oldest first. |

### shipmentBooking

| ID | Statement |
|:---|:---|
| REQ-010 | The shop shall book a collection with the carrier for every packed order. |

## orderStatus

| ID | Statement |
|:---|:---|
| REQ-011 | The shop shall show customers the status of each of their orders. |

## orderEmails

| ID | Statement | Rationale |
|:---|:---|:---|
| REQ-012 | The shop shall email the customer when an order is placed, and again when it ships. | Customers who know where their order is do not write to ask. |

## availability

| ID | Statement | Rationale |
|:---|:---|:---|
| REQ-013 | The shop shall be open for orders 99.9 % of every month. | Every hour the shop is down loses that hour's orders. |

## peakLoad

| ID | Statement | Rationale |
|:---|:---|:---|
| REQ-014 | The shop shall take ten times its usual number of orders at a peak, with no one changing how it is set up. | A sale, or a mention in the press, brings peaks no one schedules. |

## Traceability

| ID | Requirement | Satisfied by |
|:---|:---|:---|
| [REQ-001](#productsearch) | productSearch | [CatalogueFunction](../implementation.sysml#L111) |
| [REQ-002](#cataloguesize) | catalogueSize | [ProductTable](../implementation.sysml#L118) |
| [REQ-003](#guestcart) | guestCart | [StorefrontApp](../implementation.sysml#L93) |
| [REQ-004](#paidorders) | paidOrders | [OrderFunction](../implementation.sysml#L130) |
| [REQ-005](#nocarddata) | &emsp;noCardData | [PaymentFunction](../implementation.sysml#L140) |
| [REQ-006](#quickconfirmation) | &emsp;quickConfirmation | [OrderFunction](../implementation.sysml#L130) |
| [REQ-007](#orderfulfilment) | orderFulfilment | [FulfilmentFunction](../implementation.sysml#L159) |
| [REQ-008](#stockreservation) | &emsp;stockReservation | through [REQ-007](#orderfulfilment) |
| [REQ-009](#picklist) | &emsp;pickList | through [REQ-007](#orderfulfilment) |
| [REQ-010](#shipmentbooking) | &emsp;shipmentBooking | through [REQ-007](#orderfulfilment) |
| [REQ-011](#orderstatus) | orderStatus | [StorefrontApp](../implementation.sysml#L93) |
| [REQ-012](#orderemails) | orderEmails | [NotifyFunction](../implementation.sysml#L167) |
| [REQ-013](#availability) | availability | [S3](../implementation.sysml#L17), [Lambda](../implementation.sysml#L20), [DynamoDB](../implementation.sysml#L24) |
| [REQ-014](#peakload) | peakLoad | [Lambda](../implementation.sysml#L20), [DynamoDB](../implementation.sysml#L24) |

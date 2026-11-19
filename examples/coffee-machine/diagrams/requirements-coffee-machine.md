# Requirements — Coffee Machine

<sub>Rendered by ortho from [requirements.sysml](../requirements.sysml). Edit the model, not this file.</sub>

What the coffee machine shall do for a household making espresso and cappuccino at home.

15 requirements, all satisfied.

## espressoTime

| ID | Statement | Rationale |
|:---|:---|:---|
| REQ-001 | The machine shall deliver an espresso within 60 s of the user's choice, starting from standby. | A home machine is used in the rush before work; a minute is as long as its users will wait. |

### heatUp

| ID | Statement |
|:---|:---|
| REQ-002 | The machine shall heat its brewing water from 20 °C to 92 °C within 30 s. |

### brewTemperature

| ID | Statement | Rationale |
|:---|:---|:---|
| REQ-003 | The machine shall hold the brewing water within 2 °C of 92 °C throughout an extraction. | Below about 90 °C an espresso tastes sour, and above 96 °C bitter. |

### brewPressure

| ID | Statement | Rationale |
|:---|:---|:---|
| REQ-004 | The machine shall extract at 9 bar, plus or minus 1 bar, at the coffee bed. | Espresso is extracted at about 9 bar: much less leaves it thin, much more makes it bitter. |

## freshGround

| ID | Statement | Rationale |
|:---|:---|:---|
| REQ-005 | The machine shall grind whole beans for each shot, just before it is brewed. | Ground coffee loses most of its aroma within a quarter of an hour. |

## doseRange

| ID | Statement |
|:---|:---|
| REQ-006 | The machine shall dose between 7 g and 16 g of ground coffee for a shot, as the recipe sets. |

## milkFromCarton

| ID | Statement | Rationale |
|:---|:---|:---|
| REQ-007 | The machine shall froth milk drawn straight from the user's own carton, without a milk container of its own. | A milk container has to be emptied, cleaned and kept cold every day; a tube in the carton needs only a rinse. |

## waterCapacity

| ID | Statement | Rationale |
|:---|:---|:---|
| REQ-008 | The machine shall hold at least 1.5 L of water. | About a day's coffee for a household of four, so the tank needs filling once a day at most. |

## beanCapacity

| ID | Statement | Rationale |
|:---|:---|:---|
| REQ-009 | The machine shall hold at least 250 g of coffee beans. | The smallest common bag of beans then empties into it in one go. |

## beverageChoice

| ID | Statement |
|:---|:---|
| REQ-010 | The machine shall let the user choose an espresso or a cappuccino on a touch display at least 100 mm across. |

## safety

| ID | Statement |
|:---|:---|
| REQ-011 | The machine shall protect its user, and itself, from hot water, steam and overheating. |

### brewGroupInterlock

| ID | Statement | Rationale |
|:---|:---|:---|
| REQ-012 | The machine shall keep the brew group shut while it is under pressure. | Opened under pressure, it would spray scalding water and grounds. |

### dryRunCutoff

| ID | Statement | Rationale |
|:---|:---|:---|
| REQ-013 | The machine shall switch the heater off within 2 s when no water flows through it. | Heated dry, a thermoblock overheats within seconds and can damage the machine. |

## mainsPower

| ID | Statement | Rationale |
|:---|:---|:---|
| REQ-014 | The machine shall run from a 230 V, 50 Hz household socket, drawing at most 1.5 kW. | With a 2 kW kettle beside it, both stay within a 16 A kitchen circuit. |

## counterWidth

| ID | Statement | Rationale |
|:---|:---|:---|
| REQ-015 | The machine shall be no wider than 300 mm. | It then fits the space of the narrowest standard kitchen cabinet. |

## Traceability

| ID | Requirement | Satisfied by |
|:---|:---|:---|
| [REQ-001](#espressotime) | espressoTime | [CoffeeMachine](../physical.sysml#L166) |
| [REQ-002](#heatup) | &emsp;heatUp | [Thermoblock](../physical.sysml#L73) |
| [REQ-003](#brewtemperature) | &emsp;brewTemperature | [TemperatureController](../implementation.sysml#L80) |
| [REQ-004](#brewpressure) | &emsp;brewPressure | [WaterPump](../physical.sysml#L63) |
| [REQ-005](#freshground) | freshGround | [RecipeEngine](../implementation.sysml#L62), [Grinder](../physical.sysml#L93) |
| [REQ-006](#doserange) | doseRange | [BrewController](../implementation.sysml#L88), [BrewUnit](../physical.sysml#L103) |
| [REQ-007](#milkfromcarton) | milkFromCarton | [MilkTube](../physical.sysml#L116), [SteamWand](../physical.sysml#L127) |
| [REQ-008](#watercapacity) | waterCapacity | [WaterTank](../physical.sysml#L55) |
| [REQ-009](#beancapacity) | beanCapacity | [BeanHopper](../physical.sysml#L85) |
| [REQ-010](#beveragechoice) | beverageChoice | [UiApp](../implementation.sysml#L55), [TouchDisplay](../physical.sysml#L149) |
| [REQ-011](#safety) | safety | [CoffeeMachine](../physical.sysml#L166) |
| [REQ-012](#brewgroupinterlock) | &emsp;brewGroupInterlock | [BrewController](../implementation.sysml#L88) |
| [REQ-013](#dryruncutoff) | &emsp;dryRunCutoff | [TemperatureController](../implementation.sysml#L80) |
| [REQ-014](#mainspower) | mainsPower | [MainsSupply](../physical.sysml#L157) |
| [REQ-015](#counterwidth) | counterWidth | [CoffeeMachine](../physical.sysml#L166) |

# Requirements — Coffee Machine

<sub>Rendered by ortho from [requirements.sysml](../requirements.sysml). Edit the model, not this file.</sub>

What the coffee machine shall do for a household making espresso and cappuccino at home.

15 requirements, all satisfied.

## Espresso time

| ID | Statement | Rationale |
|:---|:---|:---|
| REQ-1 | The machine shall deliver an espresso within 60 s of the user's choice, starting from standby. | A home machine is used in the rush before work; a minute is as long as its users will wait. |

### Heat-up time

| ID | Statement |
|:---|:---|
| REQ-1.1 | The machine shall heat its brewing water from 20 °C to 92 °C within 30 s. |

### Brew temperature

| ID | Statement | Rationale |
|:---|:---|:---|
| REQ-1.2 | The machine shall hold the brewing water within 2 °C of 92 °C throughout an extraction. | Below about 90 °C an espresso tastes sour, and above 96 °C bitter. |

### Brew pressure

| ID | Statement | Rationale |
|:---|:---|:---|
| REQ-1.3 | The machine shall extract at 9 bar, plus or minus 1 bar, at the coffee bed. | Espresso is extracted at about 9 bar: much less leaves it thin, much more makes it bitter. |

## Freshly ground coffee

| ID | Statement | Rationale |
|:---|:---|:---|
| REQ-2 | The machine shall grind whole beans for each shot, just before it is brewed. | Ground coffee loses most of its aroma within a quarter of an hour. |

## Dose range

| ID | Statement |
|:---|:---|
| REQ-3 | The machine shall dose between 7 g and 16 g of ground coffee for a shot, as the recipe sets. |

## Milk from the carton

| ID | Statement | Rationale |
|:---|:---|:---|
| REQ-4 | The machine shall froth milk drawn straight from the user's own carton, without a milk container of its own. | A milk container has to be emptied, cleaned and kept cold every day; a tube in the carton needs only a rinse. |

## Water capacity

| ID | Statement | Rationale |
|:---|:---|:---|
| REQ-5 | The machine shall hold at least 1.5 L of water. | About a day's coffee for a household of four, so the tank needs filling once a day at most. |

## Bean capacity

| ID | Statement | Rationale |
|:---|:---|:---|
| REQ-6 | The machine shall hold at least 250 g of coffee beans. | The smallest common bag of beans then empties into it in one go. |

## Choice of beverage

| ID | Statement |
|:---|:---|
| REQ-7 | The machine shall let the user choose an espresso or a cappuccino on a touch display at least 100 mm across. |

## Safety

| ID | Statement |
|:---|:---|
| REQ-8 | The machine shall protect its user, and itself, from hot water, steam and overheating. |

### Brew group interlock

| ID | Statement | Rationale |
|:---|:---|:---|
| REQ-8.1 | The machine shall keep the brew group shut while it is under pressure. | Opened under pressure, it would spray scalding water and grounds. |

### Dry-run cut-off

| ID | Statement | Rationale |
|:---|:---|:---|
| REQ-8.2 | The machine shall switch the heater off within 2 s when no water flows through it. | Heated dry, a thermoblock overheats within seconds and can damage the machine. |

## Mains power

| ID | Statement | Rationale |
|:---|:---|:---|
| REQ-9 | The machine shall run from a 230 V, 50 Hz household socket, drawing at most 1.5 kW. | With a 2 kW kettle beside it, both stay within a 16 A kitchen circuit. |

## Machine width

| ID | Statement | Rationale |
|:---|:---|:---|
| REQ-10 | The machine shall be no wider than 300 mm. | It then fits the space of the narrowest standard kitchen cabinet. |

## Traceability

| ID | Requirement | Satisfied by |
|:---|:---|:---|
| [REQ-1](#espresso-time) | Espresso time | [CoffeeMachine](../physical.sysml#L166) |
| [REQ-1.1](#heat-up-time) | &emsp;Heat-up time | [Thermoblock](../physical.sysml#L73) |
| [REQ-1.2](#brew-temperature) | &emsp;Brew temperature | [TemperatureController](../implementation.sysml#L80) |
| [REQ-1.3](#brew-pressure) | &emsp;Brew pressure | [WaterPump](../physical.sysml#L63) |
| [REQ-2](#freshly-ground-coffee) | Freshly ground coffee | [RecipeEngine](../implementation.sysml#L62), [Grinder](../physical.sysml#L93) |
| [REQ-3](#dose-range) | Dose range | [BrewController](../implementation.sysml#L88), [BrewUnit](../physical.sysml#L103) |
| [REQ-4](#milk-from-the-carton) | Milk from the carton | [MilkTube](../physical.sysml#L116), [SteamWand](../physical.sysml#L127) |
| [REQ-5](#water-capacity) | Water capacity | [WaterTank](../physical.sysml#L55) |
| [REQ-6](#bean-capacity) | Bean capacity | [BeanHopper](../physical.sysml#L85) |
| [REQ-7](#choice-of-beverage) | Choice of beverage | [UiApp](../implementation.sysml#L55), [TouchDisplay](../physical.sysml#L149) |
| [REQ-8](#safety) | Safety | [CoffeeMachine](../physical.sysml#L166) |
| [REQ-8.1](#brew-group-interlock) | &emsp;Brew group interlock | [BrewController](../implementation.sysml#L88) |
| [REQ-8.2](#dry-run-cut-off) | &emsp;Dry-run cut-off | [TemperatureController](../implementation.sysml#L80) |
| [REQ-9](#mains-power) | Mains power | [MainsSupply](../physical.sysml#L157) |
| [REQ-10](#machine-width) | Machine width | [CoffeeMachine](../physical.sysml#L166) |

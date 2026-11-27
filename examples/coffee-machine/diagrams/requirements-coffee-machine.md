# Requirements — Coffee Machine

<sub>Rendered by ortho from [requirements.sysml](../requirements.sysml). Edit the model, not this file.</sub>

What the coffee machine shall do for a household making espresso and cappuccino at home.

18 requirements, all satisfied, all verified. To be resolved: [REQ-1.1](#heat-up-time), [REQ-7](#choice-of-beverage). To be confirmed: [PART-3](#grind-time).

## SystemRequirements

Stated of the machine as a black box, like the use cases: no part or program is named here. Each part or module that meets a requirement says so where it is declared, with \`satisfy\`, beside its \`perform\`.

### Espresso time

| ID | Statement | Rationale | Status |
|:---|:---|:---|:---|
| REQ-1 | While the machine is in standby, when the user chooses an espresso, the machine shall deliver it within 60 s. | A home machine is used in the rush before work; a minute is as long as its users will wait. | — |

#### Heat-up time

| ID | Statement | Status |
|:---|:---|:---|
| REQ-1.1 | The machine shall heat its brewing water from 20 °C to 92 °C within 30 s. | TBR |

#### Brew temperature

| ID | Statement | Rationale | Status |
|:---|:---|:---|:---|
| REQ-1.2 | While it extracts a shot, the machine shall hold the brewing water within 2 °C of 92 °C. | Below about 90 °C an espresso tastes sour, and above 96 °C bitter. | — |

#### Brew pressure

| ID | Statement | Rationale | Status |
|:---|:---|:---|:---|
| REQ-1.3 | While it extracts a shot, the machine shall hold the pressure at the coffee bed at 9 bar, plus or minus 1 bar. | Espresso is extracted at about 9 bar: much less leaves it thin, much more makes it bitter. | — |

### Freshly ground coffee

| ID | Statement | Rationale | Status |
|:---|:---|:---|:---|
| REQ-2 | When it brews a shot, the machine shall first grind whole beans for it. | Ground coffee loses most of its aroma within a quarter of an hour. | — |

### Dose range

| ID | Statement | Status |
|:---|:---|:---|
| REQ-3 | The machine shall dose between 7 g and 16 g of ground coffee for a shot, as the recipe sets. | — |

### Milk from the carton

| ID | Statement | Rationale | Status |
|:---|:---|:---|:---|
| REQ-4 | The machine shall froth milk drawn straight from the user's own carton. | A milk container has to be emptied, cleaned and kept cold every day; a tube in the carton needs only a rinse. | — |

### Water capacity

| ID | Statement | Rationale | Status |
|:---|:---|:---|:---|
| REQ-5 | The machine shall hold at least 1.5 L of water. | About a day's coffee for a household of four, so the tank needs filling once a day at most. | — |

### Bean capacity

| ID | Statement | Rationale | Status |
|:---|:---|:---|:---|
| REQ-6 | The machine shall hold at least 250 g of coffee beans. | The smallest common bag of beans then empties into it in one go. | — |

### Choice of beverage

| ID | Statement | Status |
|:---|:---|:---|
| REQ-7 | The machine shall let the user choose an espresso or a cappuccino on a touch display at least 100 mm across. | TBR |

### Safety

| ID | Statement | Status |
|:---|:---|:---|
| REQ-8 | The machine shall protect its user, and itself, from hot water, steam and overheating. | — |

#### Brew group interlock

| ID | Statement | Rationale | Status |
|:---|:---|:---|:---|
| REQ-8.1 | While the brew group is under pressure, the machine shall keep it shut. | Opened under pressure, it would spray scalding water and grounds. | — |

#### Dry-run cut-off

| ID | Statement | Rationale | Status |
|:---|:---|:---|:---|
| REQ-8.2 | If no water flows through the heater, then the machine shall switch the heater off within 2 s. | Heated dry, a thermoblock overheats within seconds and can damage the machine. | — |

### Mains power

| ID | Statement | Rationale | Status |
|:---|:---|:---|:---|
| REQ-9 | The machine shall run from a 230 V, 50 Hz household socket, drawing at most 1.5 kW. | With a 2 kW kettle beside it, both stay within a 16 A kitchen circuit. | — |

### Machine width

| ID | Statement | Rationale | Status |
|:---|:---|:---|:---|
| REQ-10 | The machine shall be at most 300 mm wide. | It then fits the space of the narrowest standard kitchen cabinet. | — |

## Part requirements

What each part shall do, derived from what the machine shall do.

### Heater power

| ID | Statement | Status |
|:---|:---|:---|
| PART-1 | The thermoblock shall draw at most 1300 W. | — |

### Pump power

| ID | Statement | Status |
|:---|:---|:---|
| PART-2 | The pump shall draw at most 60 W. | — |

### Grind time

| ID | Statement | Status |
|:---|:---|:---|
| PART-3 | The grinder shall grind a 16 g dose within 15 s. | TBC |

## Traceability

| ID | Requirement | Derived from | Satisfied by | Verified by |
|:---|:---|:---|:---|:---|
| [REQ-1](#espresso-time) | Espresso time | — | [CoffeeMachine](../physical.sysml#L169) | [Espresso timing test](../verification.sysml#L11) (Test) |
| [REQ-1.1](#heat-up-time) | &emsp;Heat-up time | — | [Thermoblock](../physical.sysml#L74) | [Espresso timing test](../verification.sysml#L11) (Test) |
| [REQ-1.2](#brew-temperature) | &emsp;Brew temperature | — | [TemperatureController](../implementation.sysml#L80) | [Brew test](../verification.sysml#L22) (Test) |
| [REQ-1.3](#brew-pressure) | &emsp;Brew pressure | — | [WaterPump](../physical.sysml#L63) | [Brew test](../verification.sysml#L22) (Test) |
| [REQ-2](#freshly-ground-coffee) | Freshly ground coffee | — | [RecipeEngine](../implementation.sysml#L62), [Grinder](../physical.sysml#L95) | [Grind and dose test](../verification.sysml#L33) (Test) |
| [REQ-3](#dose-range) | Dose range | — | [BrewController](../implementation.sysml#L88), [BrewUnit](../physical.sysml#L106) | [Grind and dose test](../verification.sysml#L33) (Test) |
| [REQ-4](#milk-from-the-carton) | Milk from the carton | — | [MilkTube](../physical.sysml#L119), [SteamWand](../physical.sysml#L130) | [Milk and menu demonstration](../verification.sysml#L45) (Demonstration) |
| [REQ-5](#water-capacity) | Water capacity | — | [WaterTank](../physical.sysml#L55) | [Capacity inspection](../verification.sysml#L56) (Inspection) |
| [REQ-6](#bean-capacity) | Bean capacity | — | [BeanHopper](../physical.sysml#L87) | [Capacity inspection](../verification.sysml#L56) (Inspection) |
| [REQ-7](#choice-of-beverage) | Choice of beverage | — | [UiApp](../implementation.sysml#L55), [TouchDisplay](../physical.sysml#L152) | [Milk and menu demonstration](../verification.sysml#L45) (Demonstration) |
| [REQ-8](#safety) | Safety | — | [CoffeeMachine](../physical.sysml#L169) | [Safety test](../verification.sysml#L67) (Test) |
| [REQ-8.1](#brew-group-interlock) | &emsp;Brew group interlock | — | [BrewController](../implementation.sysml#L88) | [Safety test](../verification.sysml#L67) (Test) |
| [REQ-8.2](#dry-run-cut-off) | &emsp;Dry-run cut-off | — | [TemperatureController](../implementation.sysml#L80) | [Safety test](../verification.sysml#L67) (Test) |
| [REQ-9](#mains-power) | Mains power | — | [MainsSupply](../physical.sysml#L160) | [Power analysis](../verification.sysml#L79) (Analysis, Test) |
| [REQ-10](#machine-width) | Machine width | — | [CoffeeMachine](../physical.sysml#L169) | [Capacity inspection](../verification.sysml#L56) (Inspection) |
| [PART-1](#heater-power) | Heater power | [REQ-9](#mains-power) | [Thermoblock](../physical.sysml#L74) | [Power analysis](../verification.sysml#L79) (Analysis, Test) |
| [PART-2](#pump-power) | Pump power | [REQ-9](#mains-power) | [WaterPump](../physical.sysml#L63) | [Power analysis](../verification.sysml#L79) (Analysis, Test) |
| [PART-3](#grind-time) | Grind time | [REQ-1](#espresso-time) | [Grinder](../physical.sysml#L95) | [Grind and dose test](../verification.sysml#L33) (Test) |

# Requirements — Survey Drone

<sub>Rendered by ortho from [requirements.sysml](../requirements.sysml). Edit the model, not this file.</sub>

What the survey drone system shall do for an operator surveying an area from the air.

15 requirements, all satisfied.

## Survey planning

| ID | Statement | Rationale |
|:---|:---|:---|
| REQ-1 | The system shall let the operator plan a survey by marking its area on a map, and shall fly no route the operator has not approved. | The operator stays responsible for where the drone flies. |

### Leg coverage

| ID | Statement | Rationale |
|:---|:---|:---|
| REQ-1.1 | The system shall cover a marked area with survey legs whose camera footprints overlap by at least 70 % from one leg to the next. | Stitching the images into one map needs that much overlap. |

## Progress display

| ID | Statement |
|:---|:---|
| REQ-2 | The system shall show the operator the drone's position, and how much of the survey it has flown, at least once a second. |

## Live video

| ID | Statement | Rationale |
|:---|:---|:---|
| REQ-3 | The system shall show the operator the camera's video at most 500 ms after it is captured. | The operator judges what the drone sees from the video; an older picture misleads. |

## Automatic survey

| ID | Statement |
|:---|:---|
| REQ-4 | The drone shall fly an approved survey from take-off to landing with no one at the controls. |

### Track accuracy

| ID | Statement | Rationale |
|:---|:---|:---|
| REQ-4.1 | The drone shall keep within 2 m of its planned track, in winds up to 10 m/s. | Legs that drift apart leave gaps in the overlap the map needs. |

## Endurance

| ID | Statement | Rationale |
|:---|:---|:---|
| REQ-5 | The drone shall fly for at least 30 min on one battery, carrying its camera and both computers. | A typical site is then surveyed in one flight, with no seam in the map where a battery was changed. |

## Safe flight

| ID | Statement |
|:---|:---|
| REQ-6 | The drone shall end its flight safely when an obstacle blocks its route, its radio link is lost or its battery runs low. |

### Obstacle avoidance

| ID | Statement | Rationale |
|:---|:---|:---|
| REQ-6.1 | The drone shall detect an obstacle in its path at least 15 m ahead, and stop before it. | Power lines, cranes and trees are not on the operator's map. |

#### Onboard detection

| ID | Statement | Rationale |
|:---|:---|:---|
| REQ-6.1.1 | The drone shall detect obstacles with its own computers, using neither the radio link nor the ground station. | Detection on the ground would come too late, and stop when the link does. |

### Lost link

| ID | Statement |
|:---|:---|
| REQ-6.2 | The drone shall return to its launch point and land when the radio link has been lost for 5 s. |

### Low battery

| ID | Statement |
|:---|:---|
| REQ-6.3 | The drone shall return to its launch point once its battery holds only the charge to get there, with 20 % in reserve. |

## Link range

| ID | Statement | Rationale |
|:---|:---|:---|
| REQ-7 | The radio link shall carry the video and the telemetry up to 2 km from the ground station, in line of sight. | The operator can then stand at one edge of a site and reach its far side. |

## Link encryption

| ID | Statement | Rationale |
|:---|:---|:---|
| REQ-8 | The system shall encrypt and authenticate everything that crosses the radio link. | Video of private land must not be readable, nor commands to the drone forgeable, by anyone within radio range. |

## Flight record

| ID | Statement | Rationale |
|:---|:---|:---|
| REQ-9 | The drone shall record every flight's telemetry and health reports, to be read after landing. | An incident has to be traceable to its cause. |

## Traceability

| ID | Requirement | Satisfied by |
|:---|:---|:---|
| [REQ-1](#survey-planning) | Survey planning | [SurveyPlanner](../implementation.sysml#L120) |
| [REQ-1.1](#leg-coverage) | &emsp;Leg coverage | [RoutePlanner](../implementation.sysml#L142) |
| [REQ-2](#progress-display) | Progress display | [SurveyPlanner](../implementation.sysml#L120), [MavlinkBridge](../implementation.sysml#L196) |
| [REQ-3](#live-video) | Live video | [VideoStreamer](../implementation.sysml#L159) |
| [REQ-4](#automatic-survey) | Automatic survey | [MissionExecutor](../implementation.sysml#L222) |
| [REQ-4.1](#track-accuracy) | &emsp;Track accuracy | [StateEstimator](../implementation.sysml#L234), [FlightControl](../implementation.sysml#L243), [GpsModule](../physical.sysml#L97) |
| [REQ-5](#endurance) | Endurance | [LiPoBattery](../physical.sysml#L60) |
| [REQ-6](#safe-flight) | Safe flight | [SurveyDrone](../physical.sysml#L123) |
| [REQ-6.1](#obstacle-avoidance) | &emsp;Obstacle avoidance | [ObstacleAvoidance](../implementation.sysml#L152), [WorldModel](../implementation.sysml#L167) |
| [REQ-6.1.1](#onboard-detection) | &emsp;&emsp;Onboard detection | [GpuComputer](../physical.sysml#L80) |
| [REQ-6.2](#lost-link) | &emsp;Lost link | [MissionExecutor](../implementation.sysml#L222) |
| [REQ-6.3](#low-battery) | &emsp;Low battery | [MissionExecutor](../implementation.sysml#L222) |
| [REQ-7](#link-range) | Link range | [IpRadio](../physical.sysml#L107) |
| [REQ-8](#link-encryption) | Link encryption | [WireGuardTunnel](../implementation.sysml#L206) |
| [REQ-9](#flight-record) | Flight record | [Diagnostics](../implementation.sysml#L176) |

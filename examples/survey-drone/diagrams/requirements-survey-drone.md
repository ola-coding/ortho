# Requirements — Survey Drone

<sub>Rendered by ortho from [requirements.sysml](../requirements.sysml). Edit the model, not this file.</sub>

What the survey drone system shall do for an operator surveying an area from the air.

15 requirements, all satisfied.

## surveyPlanning

| ID | Statement | Rationale |
|:---|:---|:---|
| REQ-001 | The system shall let the operator plan a survey by marking its area on a map, and shall fly no route the operator has not approved. | The operator stays responsible for where the drone flies. |

### legCoverage

| ID | Statement | Rationale |
|:---|:---|:---|
| REQ-002 | The system shall cover a marked area with survey legs whose camera footprints overlap by at least 70 % from one leg to the next. | Stitching the images into one map needs that much overlap. |

## progressDisplay

| ID | Statement |
|:---|:---|
| REQ-003 | The system shall show the operator the drone's position, and how much of the survey it has flown, at least once a second. |

## liveVideo

| ID | Statement | Rationale |
|:---|:---|:---|
| REQ-004 | The system shall show the operator the camera's video at most 500 ms after it is captured. | The operator judges what the drone sees from the video; an older picture misleads. |

## automaticSurvey

| ID | Statement |
|:---|:---|
| REQ-005 | The drone shall fly an approved survey from take-off to landing with no one at the controls. |

### trackAccuracy

| ID | Statement | Rationale |
|:---|:---|:---|
| REQ-006 | The drone shall keep within 2 m of its planned track, in winds up to 10 m/s. | Legs that drift apart leave gaps in the overlap the map needs. |

## endurance

| ID | Statement | Rationale |
|:---|:---|:---|
| REQ-007 | The drone shall fly for at least 30 min on one battery, carrying its camera and both computers. | A typical site is then surveyed in one flight, with no seam in the map where a battery was changed. |

## safeFlight

| ID | Statement |
|:---|:---|
| REQ-008 | The drone shall end its flight safely when an obstacle blocks its route, its radio link is lost or its battery runs low. |

### obstacleAvoidance

| ID | Statement | Rationale |
|:---|:---|:---|
| REQ-009 | The drone shall detect an obstacle in its path at least 15 m ahead, and stop before it. | Power lines, cranes and trees are not on the operator's map. |

#### onboardDetection

| ID | Statement | Rationale |
|:---|:---|:---|
| REQ-010 | The drone shall detect obstacles with its own computers, using neither the radio link nor the ground station. | Detection on the ground would come too late, and stop when the link does. |

### lostLink

| ID | Statement |
|:---|:---|
| REQ-011 | The drone shall return to its launch point and land when the radio link has been lost for 5 s. |

### lowBattery

| ID | Statement |
|:---|:---|
| REQ-012 | The drone shall return to its launch point once its battery holds only the charge to get there, with 20 % in reserve. |

## linkRange

| ID | Statement | Rationale |
|:---|:---|:---|
| REQ-013 | The radio link shall carry the video and the telemetry up to 2 km from the ground station, in line of sight. | The operator can then stand at one edge of a site and reach its far side. |

## linkEncryption

| ID | Statement | Rationale |
|:---|:---|:---|
| REQ-014 | The system shall encrypt and authenticate everything that crosses the radio link. | Video of private land must not be readable, nor commands to the drone forgeable, by anyone within radio range. |

## flightRecord

| ID | Statement | Rationale |
|:---|:---|:---|
| REQ-015 | The drone shall record every flight's telemetry and health reports, to be read after landing. | An incident has to be traceable to its cause. |

## Traceability

| ID | Requirement | Satisfied by |
|:---|:---|:---|
| [REQ-001](#surveyplanning) | surveyPlanning | [SurveyPlanner](../implementation.sysml#L120) |
| [REQ-002](#legcoverage) | &emsp;legCoverage | [RoutePlanner](../implementation.sysml#L142) |
| [REQ-003](#progressdisplay) | progressDisplay | [SurveyPlanner](../implementation.sysml#L120), [MavlinkBridge](../implementation.sysml#L196) |
| [REQ-004](#livevideo) | liveVideo | [VideoStreamer](../implementation.sysml#L159) |
| [REQ-005](#automaticsurvey) | automaticSurvey | [MissionExecutor](../implementation.sysml#L222) |
| [REQ-006](#trackaccuracy) | &emsp;trackAccuracy | [StateEstimator](../implementation.sysml#L234), [FlightControl](../implementation.sysml#L243), [GpsModule](../physical.sysml#L97) |
| [REQ-007](#endurance) | endurance | [LiPoBattery](../physical.sysml#L60) |
| [REQ-008](#safeflight) | safeFlight | [SurveyDrone](../physical.sysml#L123) |
| [REQ-009](#obstacleavoidance) | &emsp;obstacleAvoidance | [ObstacleAvoidance](../implementation.sysml#L152), [WorldModel](../implementation.sysml#L167) |
| [REQ-010](#onboarddetection) | &emsp;&emsp;onboardDetection | [GpuComputer](../physical.sysml#L80) |
| [REQ-011](#lostlink) | &emsp;lostLink | [MissionExecutor](../implementation.sysml#L222) |
| [REQ-012](#lowbattery) | &emsp;lowBattery | [MissionExecutor](../implementation.sysml#L222) |
| [REQ-013](#linkrange) | linkRange | [IpRadio](../physical.sysml#L107) |
| [REQ-014](#linkencryption) | linkEncryption | [WireGuardTunnel](../implementation.sysml#L206) |
| [REQ-015](#flightrecord) | flightRecord | [Diagnostics](../implementation.sysml#L176) |

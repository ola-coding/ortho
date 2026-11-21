// Physical view: the product itself, propulsion, avionics, radio and ground kit.
package Hardware {
    doc /* Topology only: what feeds and wires to what. No geometry, no frame, no
         * placement on it. */

    // A part performs the functions it does itself, and satisfies the
    // requirements it meets. The computers do both through the software
    // deployed on them, and satisfy on their own what their hardware meets.
    private import Functions::*;
    private import SystemRequirements::*;
    private import ScalarValues::*;
    private import ISQ::*;
    private import SI::*;
    private import StandardViewDefinitions::*;

    // What flows through the ports. A port def says which way its item goes;
    // the receiving end is the same port conjugated, `~PowerPort`.
    item def ElectricPower;
    item def PhaseCurrent;
    item def Throttle;
    item def GpsFix;
    item def VideoStream;
    item def Data;

    port def PowerPort { out item power : ElectricPower; }
    port def DrivePort { out item current : PhaseCurrent; }
    port def ThrottlePort { out item throttle : Throttle; }
    port def GpsPort { out item fix : GpsFix; }
    port def VideoPort { out item video : VideoStream; }
    port def DataPort { inout item data : Data; }
    port def RfPort {
        inout item data : Data;
        attribute band : FrequencyValue = 2.4E9 [Hz];
    }

    // The contract the air-to-ground link satisfies at both ends.
    interface def RadioLink {
        end air : RfPort;
        end ground : RfPort;
    }

    // --- propulsion --------------------------------------------------------
    part def PropulsionUnit {
        doc /* A brushless motor and its propeller. */
        perform survey.operateVehicle.produceThrust;

        port phases : ~DrivePort;
    }

    part def FourInOneEsc {
        doc /* One board drives all four motors, as on most quadcopters. */
        perform survey.operateVehicle.produceThrust;

        port powerIn : ~PowerPort;
        port signalIn : ~ThrottlePort;
        port phasesOut : DrivePort;
    }

    // --- power -------------------------------------------------------------
    part def LiPoBattery {
        doc /* Feeds every consumer that needs a supply rail of its own. */
        perform survey.operateVehicle.supplyPower;
        satisfy 'Endurance';

        attribute cells : Integer = 6;
        attribute energy : EnergyValue = 222 ['W⋅h'];
        port powerOut : PowerPort;
    }

    // --- computers ---------------------------------------------------------
    part def ControlComputer {
        doc /* ArduPilot-style flight controller with its inertial sensors on
             * board: drives the motors and talks MAVLink to the computer beside it. */
        port powerIn : ~PowerPort;
        port gpsIn : ~GpsPort;
        port motorOut : ThrottlePort;
        port mavlink : DataPort;
    }

    part def GpuComputer {
        doc /* Jetson-style computer with a 100 TOPS GPU, for everything above flying. */
        satisfy 'Safe flight'.'Obstacle avoidance'.'Onboard detection';

        port powerIn : ~PowerPort;
        port cameraIn : ~VideoPort;
        port autopilot : DataPort;
        port ethernet : DataPort;
    }

    // --- sensors -----------------------------------------------------------
    part def Camera {
        perform survey.perceive.captureVideo;

        port videoOut : VideoPort;
    }

    part def GpsModule {
        doc /* Receiver and antenna in one puck, as on most ArduPilot builds. It
             * takes its power over the same connector as its data. */
        perform survey.guideNavigateAndControl.navigate;
        satisfy 'Automatic survey'.'Track accuracy';

        port dataOut : GpsPort;
    }

    // --- radio link and ground kit -----------------------------------------
    part def IpRadio {
        doc /* An IP radio: the same unit is fitted at both ends of the link. */
        perform survey.communicate;
        satisfy 'Link range';

        port ethernet : DataPort;
        port rf : RfPort;
    }

    part def OperatorTablet {
        perform survey.interactWithOperator;

        port network : DataPort;
    }

    // --- assemblies --------------------------------------------------------
    part def SurveyDrone {
        // What only the drone as a whole meets.
        satisfy 'Safe flight';

        part motors : PropulsionUnit [4];
        part esc : FourInOneEsc;
        part battery : LiPoBattery;
        part fc : ControlComputer;
        part companion : GpuComputer;
        part camera : Camera;
        part gps : GpsModule;
        part radio : IpRadio;

        connect esc.phasesOut to motors.phases;
        connect battery.powerOut to esc.powerIn;
        connect battery.powerOut to fc.powerIn;
        connect battery.powerOut to companion.powerIn;
        connect fc.motorOut to esc.signalIn;
        connect gps.dataOut to fc.gpsIn;
        connect fc.mavlink to companion.autopilot;
        connect camera.videoOut to companion.cameraIn;
        connect companion.ethernet to radio.ethernet;

        // What passes one way through those connections. Data goes both ways,
        // so its lines carry no arrowhead.
        flow of PhaseCurrent from esc.phasesOut.current to motors.phases.current;
        flow of ElectricPower from battery.powerOut.power to esc.powerIn.power;
        flow of ElectricPower from battery.powerOut.power to fc.powerIn.power;
        flow of ElectricPower from battery.powerOut.power to companion.powerIn.power;
        flow of Throttle from fc.motorOut.throttle to esc.signalIn.throttle;
        flow of GpsFix from gps.dataOut.fix to fc.gpsIn.fix;
        flow of VideoStream from camera.videoOut.video to companion.cameraIn.video;
    }

    part def GroundStation {
        part tablet : OperatorTablet;
        part radio : IpRadio;

        connect tablet.network to radio.ethernet;
    }

    // The system as built: the subject of every use case, opened up.
    part def SurveyDroneSystem :> SystemUseCases::SurveyDroneSystem {
        part drone : SurveyDrone;
        part ground : GroundStation;

        interface : RadioLink connect drone.radio.rf to ground.radio.rf;
    }

    // The system itself, named once so deployment.sysml can allocate to the
    // computers inside it: a reference into a definition has to start at a usage.
    part system : SurveyDroneSystem;

    view 'Physical view — Survey Drone' : InterconnectionView {
        expose system;
    }
}

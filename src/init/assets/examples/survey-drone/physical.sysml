// Physical view: the product itself, propulsion, avionics, radio and ground kit.
package Hardware {
    doc /* Topology only: what feeds and wires to what. No geometry, no frame, no
         * placement on it. */

    // A part performs the functions it does itself. The computers perform theirs
    // through the software deployed on them.
    private import Functions::*;
    private import ScalarValues::*;
    private import ISQ::*;
    private import SI::*;
    private import StandardViewDefinitions::*;

    // What flows through the ports. A port def says which way its item goes;
    // the receiving end is the same port conjugated, `~PowerPort`.
    item def ElectricPower;
    item def MotorDrive;
    item def Signal;
    item def Data;

    port def PowerPort { out item power : ElectricPower; }
    port def DrivePort { out item drive : MotorDrive; }
    port def SignalPort { out item signal : Signal; }
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
        port signalIn : ~SignalPort;
        port phasesOut : DrivePort;
    }

    // --- power -------------------------------------------------------------
    part def LiPoBattery {
        doc /* Feeds every consumer that needs a supply rail of its own. */
        perform survey.operateVehicle.supplyPower;

        attribute cells : Integer = 6;
        attribute energy : EnergyValue = 222 ['W⋅h'];
        port powerOut : PowerPort;
    }

    // --- computers ---------------------------------------------------------
    part def ControlComputer {
        doc /* ArduPilot-style flight controller with its inertial sensors on
             * board: drives the motors and talks MAVLink to the computer beside it. */
        port powerIn : ~PowerPort;
        port gpsIn : ~SignalPort;
        port motorOut : SignalPort;
        port mavlink : DataPort;
    }

    part def GpuComputer {
        doc /* Jetson-style computer with a 100 TOPS GPU, for everything above flying. */
        port powerIn : ~PowerPort;
        port cameraIn : ~SignalPort;
        port autopilot : DataPort;
        port ethernet : DataPort;
    }

    // --- sensors -----------------------------------------------------------
    part def Camera {
        perform survey.perceive.captureVideo;

        port videoOut : SignalPort;
    }

    part def GpsModule {
        doc /* Receiver and antenna in one puck, as on most ArduPilot builds. It
             * takes its power over the same connector as its data. */
        perform survey.guideNavigateAndControl.navigate;

        port dataOut : SignalPort;
    }

    // --- radio link and ground kit -----------------------------------------
    part def IpRadio {
        doc /* An IP radio: the same unit is fitted at both ends of the link. */
        perform survey.communicate;

        port ethernet : DataPort;
        port rf : RfPort;
    }

    part def OperatorTablet {
        perform survey.interactWithOperator;

        port network : DataPort;
    }

    // --- assemblies --------------------------------------------------------
    part def SurveyDrone {
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

// Physical view: the product itself — airframe, avionics, radio and ground kit.
//
// Topology only: what connects, mounts and wires to what. No geometry, no mass
// distribution, no placement on the frame.
//
// A part carries a power port only where it needs a supply rail of its own.
// The camera, the IMU, the GPS puck and the radio take their power over the
// same connector that carries their data, so drawing a second line to each
// would state a cable that does not exist.
package Hardware {

    // Real is the spec's own value type, from its library.
    private import ScalarValues::*;

    // The functions this hardware realizes, said on the part that realizes
    // each one. Sensors and the radio carry data for functions the software
    // performs, so they realize none of their own.
    private import Functions::*;

    port def PowerPort;
    port def DataPort;
    port def DrivePort;
    port def MountPort;
    port def RfPort {
        attribute band_GHz : Real = 2.4;
    }

    // The contract the air-to-ground link satisfies at both ends.
    interface def RadioLink {
        end air : RfPort;
        end ground : RfPort;
    }

    // --- airframe and propulsion -------------------------------------------
    part def Fuselage {
        port motorArms : MountPort;
        port cameraMount : MountPort;
    }

    part def BrushlessMotor {
        perform Functions::survey.operateVehicle.produceThrust;

        port arm : MountPort;
        port phases : DrivePort;
        port shaft : MountPort;
    }

    part def Propeller {
        perform Functions::survey.operateVehicle.produceThrust;

        port hub : MountPort;
    }

    // One board drives all four motors, as on most quadcopters.
    part def FourInOneEsc {
        port powerIn : PowerPort;
        port signalIn : DataPort;
        port phasesOut : DrivePort;
    }

    // --- power -------------------------------------------------------------
    part def LiPoBattery {
        perform Functions::survey.operateVehicle.supplyPower;

        attribute cells : Real = 6;
        attribute capacity_mAh : Real = 10000;
        port powerOut : PowerPort;
    }

    // Measures battery voltage and current for the control computer, and
    // gives each consumer its own supply.
    part def PowerModule {
        perform Functions::survey.operateVehicle.supplyPower;

        port batteryIn : PowerPort;
        port escOut : PowerPort;
        port controlOut : PowerPort;
        port computeOut : PowerPort;
    }

    // --- computers ---------------------------------------------------------
    // ArduPilot-style flight controller: reads the sensors, drives the motors
    // and talks MAVLink to the computer beside it.
    part def ControlComputer {
        perform Functions::survey.operateVehicle.provideComputing;

        port powerIn : PowerPort;
        port imuIn : DataPort;
        port gpsIn : DataPort;
        port motorOut : DataPort;
        port mavlink : DataPort;
    }

    // Jetson-style computer with a GPU, for everything above flying.
    part def GpuComputer {
        perform Functions::survey.operateVehicle.provideComputing;

        attribute aiPerformance_TOPS : Real = 100;
        port powerIn : PowerPort;
        port cameraIn : DataPort;
        port autopilot : DataPort;
        port ethernet : DataPort;
    }

    // --- sensors -----------------------------------------------------------
    part def Camera {
        perform Functions::survey.perceive.captureVideo;

        port mount : MountPort;
        port videoOut : DataPort;
    }

    part def Imu {
        port dataOut : DataPort;
    }

    // Receiver and antenna in one puck, as on most ArduPilot builds.
    part def GpsModule {
        port dataOut : DataPort;
    }

    // --- radio link and ground kit -----------------------------------------
    // An IP radio: the same unit is fitted at both ends of the link.
    part def IpRadio {
        port ethernet : DataPort;
        port rf : RfPort;
    }

    part def OperatorTablet {
        port network : DataPort;
    }

    // --- assemblies --------------------------------------------------------
    part def SurveyDrone {
        port rfLink : RfPort;

        part airframe : Fuselage;
        part motors : BrushlessMotor [4];
        part props : Propeller [4];
        part esc : FourInOneEsc;
        part battery : LiPoBattery;
        part power : PowerModule;
        part fc : ControlComputer;
        part companion : GpuComputer;
        part camera : Camera;
        part imu : Imu;
        part gps : GpsModule;
        part radio : IpRadio;

        connect battery.powerOut to power.batteryIn;
        connect power.escOut to esc.powerIn;
        connect power.controlOut to fc.powerIn;
        connect power.computeOut to companion.powerIn;
        connect fc.motorOut to esc.signalIn;
        connect esc.phasesOut to motors.phases;
        connect motors.shaft to props.hub;
        connect motors.arm to airframe.motorArms;
        connect camera.mount to airframe.cameraMount;
        connect imu.dataOut to fc.imuIn;
        connect gps.dataOut to fc.gpsIn;
        connect fc.mavlink to companion.autopilot;
        connect camera.videoOut to companion.cameraIn;
        connect companion.ethernet to radio.ethernet;
        connect radio.rf to rfLink;
    }

    part def GroundStation {
        port rfLink : RfPort;

        part tablet : OperatorTablet;
        part radio : IpRadio;

        connect tablet.network to radio.ethernet;
        connect radio.rf to rfLink;
    }

    part def SurveyDroneSystem {
        part drone : SurveyDrone;
        part ground : GroundStation;

        connection : RadioLink connect drone.rfLink to ground.rfLink;
    }

    // The system itself, named once so deployment.sysml can allocate to the
    // computers inside it: a reference into a definition has to start at a
    // usage.
    part system : SurveyDroneSystem;
}

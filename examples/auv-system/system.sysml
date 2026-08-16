// System level: the logical architecture of the AUV system.
//
// Holds the interfaces that cross a domain boundary, a block per component the
// assemblies compose — ports only, no design detail — and the three assemblies
// themselves. The domains specialize these blocks to supply the design that
// realizes them, so the dependency runs domain -> system and this package
// references nothing but the requirements it traces to.
package System {

    import Requirements::*;

    // --- cross-domain interfaces ------------------------------------------
    // Electronics owns the electrical budget: voltage, current, power draw.
    port def PowerPort {
        attribute voltage_V : Real;
    }
    port def DataPort;
    port def EscPort;
    port def RfPort {
        attribute band_GHz : String = "2.4/5.8";
    }
    // Mechanics owns the physical budget: mass, geometry, mounting.
    port def MountPort;

    // --- architecture blocks ----------------------------------------------
    part def FlightController {
        port powerIn : PowerPort;
        port sensorsIn : DataPort;
        port escOut : EscPort;
        port radioLink : DataPort;
        port gimbalCtrl : DataPort;
    }

    part def Imu {
        port dataOut : DataPort;
    }

    part def GnssModule {
        port dataOut : DataPort;
    }

    part def VisionSensor {
        port dataOut : DataPort;
    }

    part def InfraredTof {
        port dataOut : DataPort;
    }

    part def Camera {
        // mount is the mechanical interface; the rest is electronic.
        port mount : MountPort;
        port ctrlIn : DataPort;
        port videoOut : DataPort;
    }

    part def Gimbal {
        port mount : MountPort;
        port ctrlIn : DataPort;
    }

    part def Motor {
        port drive : EscPort;
    }

    part def Esc {
        port ctrlIn : EscPort;
        port motorOut : EscPort;
    }

    part def Propeller;

    // The radio link technology is used on BOTH ends: the aircraft and the
    // controller each compose one transceiver.
    part def RadioTransceiver {
        port radioLink : DataPort;
        port video : DataPort;
        port rf : RfPort;
    }

    part def FlightBattery {
        port powerOut : PowerPort;
    }

    part def TouchScreen {
        port videoIn : DataPort;
    }

    part def ControlSticks {
        port ctrlOut : DataPort;
    }

    part def BatteryCharger {
        port chargePort : PowerPort;
    }

    // --- assemblies -------------------------------------------------------
    // These compose across domain boundaries, which is why they stay at system
    // level rather than being realized by any one domain.
    part def Aircraft {
        attribute takeoffWeight_g : Real = 249;
        port batteryBay : PowerPort;
        port rfLink : RfPort;

        part fc : FlightController;
        part imu : Imu;
        part gnss : GnssModule;
        part vision : VisionSensor [4];
        part tof : InfraredTof;
        part camera : Camera;
        part gimbal : Gimbal;
        part motors : Motor [4];
        part escs : Esc [4];
        part props : Propeller [4];
        part radio : RadioTransceiver;

        connect batteryBay to fc.powerIn;
        connect imu.dataOut to fc.sensorsIn;
        connect gnss.dataOut to fc.sensorsIn;
        connect vision.dataOut to fc.sensorsIn;
        connect tof.dataOut to fc.sensorsIn;
        connect fc.escOut to escs.ctrlIn;
        connect escs.motorOut to motors.drive;
        connect fc.gimbalCtrl to gimbal.ctrlIn;
        connect fc.radioLink to radio.radioLink;
        connect camera.videoOut to radio.video;
        connect gimbal.mount to camera.mount;
        connect radio.rf to rfLink;
    }

    part def RadioController {
        attribute weight_g : Real = 420;
        port rfLink : RfPort;

        part display : TouchScreen;
        part sticks : ControlSticks;
        part radio : RadioTransceiver;

        connect radio.video to display.videoIn;
        connect sticks.ctrlOut to radio.radioLink;
        connect radio.rf to rfLink;
    }

    part def AuvSystem {
        part aircraft : Aircraft;
        part controller : RadioController;
        part batteries : FlightBattery [1..3];
        part charger : BatteryCharger;

        connect batteries.powerOut to aircraft.batteryBay;
        connect batteries.powerOut to charger.chargePort;
        connect aircraft.rfLink to controller.rfLink;
    }

    // --- system-level requirement traces ----------------------------------
    // Both are properties of the whole aircraft, not of any one domain.
    satisfy Requirements::weightReq by Aircraft;
    satisfy Requirements::speedReq by Aircraft;
}

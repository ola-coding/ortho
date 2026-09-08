// Physical view: the product itself — airframe, avionics, radio and ground kit.
//
// Topology only: what connects, mounts and wires to what. No geometry, no mass
// distribution, no placement inside the shell.
package Hardware {

    port def PowerPort {
        attribute voltage_V : Real = 11.55;
    }
    port def DataPort;
    port def EscPort;
    port def MountPort;
    port def RfPort {
        attribute band_GHz : String = "2.4/5.8";
    }

    // The contract the air-to-ground link satisfies at both ends.
    interface def RadioLink {
        end air : RfPort;
        end ground : RfPort;
    }

    // --- avionics ----------------------------------------------------------
    part def SocFlightController {
        attribute socCores : Real = 4;
        port powerIn : PowerPort;
        port sensorsIn : DataPort;
        port escOut : EscPort;
        port radioLink : DataPort;
        port gimbalCtrl : DataPort;
    }

    // --- sensing -----------------------------------------------------------
    part def MemsImu {
        port dataOut : DataPort;
    }

    part def MultiBandGnss {
        attribute constellations : String = "GPS_Galileo_BeiDou";
        port dataOut : DataPort;
    }

    part def StereoVisionSensor {
        port dataOut : DataPort;
    }

    part def TofRangefinder {
        port dataOut : DataPort;
    }

    // --- payload -----------------------------------------------------------
    part def HdrCamera {
        attribute sensor : String = "1/1.3in_CMOS";
        attribute maxVideo : String = "4K60_HDR";
        port mount : MountPort;
        port ctrlIn : DataPort;
        port videoOut : DataPort;
    }

    part def ThreeAxisGimbal {
        attribute axes : Real = 3;
        port mount : MountPort;
        port ctrlIn : DataPort;
    }

    // --- propulsion --------------------------------------------------------
    part def BrushlessMotor {
        port drive : EscPort;
    }

    part def BrushlessEsc {
        port ctrlIn : EscPort;
        port motorOut : EscPort;
    }

    part def Propeller;

    // Folding blades: a design variant of the same part, and the choice that
    // lets the aircraft stow flat.
    part def FoldingPropeller :> Propeller;

    // --- radio link --------------------------------------------------------
    // The same transceiver is fitted at both ends of the link.
    part def DualBandTransceiver {
        attribute maxRange_km : Real = 20;
        port radioLink : DataPort;
        port video : DataPort;
        port rf : RfPort;
    }

    // --- power and ground equipment ----------------------------------------
    part def LiPoFlightBattery {
        attribute capacity_mAh : Real = 2590;
        attribute weight_g : Real = 80.5;
        port powerOut : PowerPort;
    }

    part def OledTouchScreen {
        attribute size_in : Real = 5.5;
        port videoIn : DataPort;
    }

    part def GimbalStick {
        port ctrlOut : DataPort;
    }

    part def MultiBayCharger {
        attribute batterySlots : Real = 3;
        port chargePort : PowerPort;
    }

    // --- assemblies --------------------------------------------------------
    part def Aircraft {
        attribute takeoffWeight_g : Real = 249;
        port batteryBay : PowerPort;
        port rfLink : RfPort;

        part fc : SocFlightController;
        part imu : MemsImu;
        part gnss : MultiBandGnss;
        part vision : StereoVisionSensor [4];
        part tof : TofRangefinder;
        part camera : HdrCamera;
        part gimbal : ThreeAxisGimbal;
        part motors : BrushlessMotor [4];
        part escs : BrushlessEsc [4];
        part props : FoldingPropeller [4];
        part radio : DualBandTransceiver;

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

        part display : OledTouchScreen;
        part sticks : GimbalStick;
        part radio : DualBandTransceiver;

        connect radio.video to display.videoIn;
        connect sticks.ctrlOut to radio.radioLink;
        connect radio.rf to rfLink;
    }

    part def AuvSystem {
        part aircraft : Aircraft;
        part controller : RadioController;
        part batteries : LiPoFlightBattery [1..3];
        part charger : MultiBayCharger;

        connect batteries.powerOut to aircraft.batteryBay;
        connect batteries.powerOut to charger.chargePort;
        connection : RadioLink connect aircraft.rfLink to controller.rfLink;
    }
}

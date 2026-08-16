// Electronic domain: avionics, sensing, radio link and power storage.
//
// Owns the electrical budget — voltage, current, power draw — and the
// signalling interfaces at system level. Each definition specializes an
// architecture block from System, so the ports are inherited and only the design
// choices are declared. The names are the design choices.
package Electronics {

    import System::*;
    import Requirements::*;

    // --- avionics ---------------------------------------------------------
    part def SocFlightController :> System::FlightController {
        attribute socCores : Real = 4;
    }

    // --- sensing ----------------------------------------------------------
    part def MemsImu :> System::Imu;

    part def MultiBandGnss :> System::GnssModule {
        attribute constellations : String = "GPS_Galileo_BeiDou";
    }

    part def StereoVisionSensor :> System::VisionSensor;

    part def TofRangefinder :> System::InfraredTof;

    // --- payload ----------------------------------------------------------
    part def HdrCamera :> System::Camera {
        attribute sensor : String = "1/1.3in_CMOS";
        attribute maxVideo : String = "4K60_HDR";
    }

    // --- propulsion drive electronics -------------------------------------
    part def BrushlessEsc :> System::Esc;

    // --- radio link -------------------------------------------------------
    part def DualBandTransceiver :> System::RadioTransceiver {
        attribute maxRange_km : Real = 20;
    }

    // --- power and ground equipment ---------------------------------------
    part def LiPoFlightBattery :> System::FlightBattery {
        attribute capacity_mAh : Real = 2590;
        attribute weight_g : Real = 80.5;
    }

    part def OledTouchScreen :> System::TouchScreen {
        attribute size_in : Real = 5.5;
        attribute brightness_nit : Real = 700;
    }

    part def MultiBayCharger :> System::BatteryCharger {
        attribute batterySlots : Real = 3;
    }

    // --- requirement traces owned by this domain --------------------------
    satisfy Requirements::flightTimeReq by LiPoFlightBattery;
    satisfy Requirements::rangeReq by DualBandTransceiver;
    satisfy Requirements::sensingReq by StereoVisionSensor;
}

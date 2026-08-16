// Phase 1 smoke model: logical view of a small quadcopter.
package DroneLogical {

    port def PowerPort {
        attribute voltage : Real;
    }

    port def CtrlPort;

    part def Drone {
        attribute mass : Real = 249;
        part fc : FlightController;
        part battery : Battery;
        part motors : Motor [4];
        connect battery.powerOut to fc.powerIn;
        connect fc.motorCtrl to motors.ctrlIn;
        satisfy massReq by Drone;
    }

    part def FlightController {
        attribute cpuFreq : Real;
        port powerIn : PowerPort;
        port motorCtrl : CtrlPort;
    }

    part def Battery {
        attribute capacity : Real = 2590;
        port powerOut : PowerPort;
    }

    part def Motor {
        port ctrlIn : CtrlPort;
    }

    part def Quadcopter :> Drone;

    requirement def MassLimit {
        attribute maxMass : Real = 249;
        subject vehicle : Drone;
    }

    requirement massReq : MassLimit;
}

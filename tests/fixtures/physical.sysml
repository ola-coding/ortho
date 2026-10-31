// Smoke model for the physical view: parts, ports and connections of a small quadcopter.
package DronePhysical {
    private import ScalarValues::*;
    private import StandardViewDefinitions::*;

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
        satisfy powerReq by battery;
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

    requirement def PowerBudget {
        subject source : Battery;
        attribute minCapacity : Real = 2000;
    }

    requirement powerReq : PowerBudget;

    part quadcopter : Quadcopter;

    view 'Physical view — DronePhysical' : InterconnectionView {
        expose quadcopter;
    }
}

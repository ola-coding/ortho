// Phase 4 smoke model: process view — message exchange during a mission.
package DroneProcess {

    part def Pilot;
    part def FlightControllerSw;
    part def NavigationSw;
    part def MotorControl;

    part pilot : Pilot;
    part fc : FlightControllerSw;
    part nav : NavigationSw;
    part motors : MotorControl;

    action def ExecuteMission {
        message cmdTakeOff of TakeOffCommand from pilot to fc;
        then message planRoute from fc to nav;
        then message computePath from nav to nav;
        then message waypoints from nav to fc;
        then message motorSetpoints from fc to motors;
        then message telemetry from fc to pilot;
    }
}

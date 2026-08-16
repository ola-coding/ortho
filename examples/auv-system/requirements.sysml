// System requirements for the autonomous unmanned aerial vehicle.
// Self-contained: subjects are untyped so the requirements package has no
// dependency on the design packages (the design satisfies these, not vice versa).
package Requirements {

    requirement def MaxTakeoffWeight {
        subject drone;
        attribute maxWeight_g : Real = 249;
    }

    requirement def MinFlightTime {
        subject drone;
        attribute minMinutes : Real = 34;
    }

    requirement def MaxHorizontalSpeed {
        subject drone;
        attribute maxSpeed_ms : Real = 16;
    }

    requirement def VideoTransmissionRange {
        subject link;
        attribute minRange_km : Real = 20;
    }

    requirement def OmnidirectionalSensing {
        subject sensors;
        attribute coverage : String = "omnidirectional";
    }

    requirement def SubjectTracking {
        subject tracker;
        attribute mode : String = "target_tracking_360";
    }

    requirement weightReq : MaxTakeoffWeight;
    requirement flightTimeReq : MinFlightTime;
    requirement speedReq : MaxHorizontalSpeed;
    requirement rangeReq : VideoTransmissionRange;
    requirement sensingReq : OmnidirectionalSensing;
    requirement trackingReq : SubjectTracking;
}

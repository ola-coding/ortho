// Verification: how each requirement is shown to be met, and by which method.
package SystemVerification {
    doc /* Each verification case names the requirements it shows are met with
         * `verify`, in its objective, and its method with `VerificationMethod`:
         * inspection, analysis, demonstration or test. A test of a requirement
         * does not verify those nested in it, so each is named on its own. */
    private import VerificationCases::*;
    private import SystemRequirements::*;
    private import Hardware::SurveyDroneSystem;

    verification def 'Planning demonstration' {
        doc /* Plan a survey by marking an area on the map, approve it, and watch
             * its progress on the ground station. */
        @VerificationMethod { kind = VerificationMethodKind::demo; }
        subject system : SurveyDroneSystem;
        objective {
            verify 'Survey planning';
            verify 'Progress display';
        }
    }

    verification def 'Leg coverage analysis' {
        doc /* Work out the camera footprints of planned legs over areas of several
             * shapes, and the overlap between neighbouring legs. */
        @VerificationMethod { kind = VerificationMethodKind::analyze; }
        subject system : SurveyDroneSystem;
        objective {
            verify 'Survey planning'.'Leg coverage';
        }
    }

    verification def 'Video latency test' {
        doc /* Film a running clock and compare it with the clock shown on the
             * ground station. */
        @VerificationMethod { kind = VerificationMethodKind::test; }
        subject system : SurveyDroneSystem;
        objective {
            verify 'Live video';
        }
    }

    verification def 'Survey flight test' {
        doc /* Fly a full survey on one battery, in winds up to 10 m/s, and compare
             * the logged track with the planned one. */
        @VerificationMethod { kind = VerificationMethodKind::test; }
        subject system : SurveyDroneSystem;
        objective {
            verify 'Automatic survey';
            verify 'Automatic survey'.'Track accuracy';
            verify 'Endurance';
        }
    }

    verification def 'Failsafe test' {
        doc /* Fly towards a mast, switch off the ground radio, and fly on until the
             * battery reaches its return charge. */
        @VerificationMethod { kind = VerificationMethodKind::test; }
        subject system : SurveyDroneSystem;
        objective {
            verify 'Safe flight';
            verify 'Safe flight'.'Obstacle avoidance';
            verify 'Safe flight'.'Lost link';
            verify 'Safe flight'.'Low battery';
        }
    }

    verification def 'Link range test' {
        doc /* Fly the drone 2 km from the ground station, in line of sight, and log
             * the video and the telemetry received. */
        @VerificationMethod { kind = VerificationMethodKind::test; }
        subject system : SurveyDroneSystem;
        objective {
            verify 'Link range';
        }
    }

    verification def 'Link security analysis' {
        doc /* Review the link's protocol, then try to read and forge its traffic
             * with a radio in range. */
        @VerificationMethod { kind = (VerificationMethodKind::analyze, VerificationMethodKind::test); }
        subject system : SurveyDroneSystem;
        objective {
            verify 'Link encryption';
        }
    }

    verification def 'Design inspection' {
        doc /* Check in the design that obstacle detection runs on the drone's own
             * computers, and that the flight record is kept on board. */
        @VerificationMethod { kind = VerificationMethodKind::inspect; }
        subject system : SurveyDroneSystem;
        objective {
            verify 'Safe flight'.'Obstacle avoidance'.'Onboard detection';
            verify 'Flight record';
        }
    }
}

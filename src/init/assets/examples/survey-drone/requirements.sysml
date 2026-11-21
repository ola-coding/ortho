// Requirements: what the system shall do, each stated once, with an ID.
package SystemRequirements {
    doc /* Stated of the system as a black box, like the use cases: no computer,
         * radio or program is named here. Each part or module that meets a
         * requirement says so where it is declared, with `satisfy`, beside its
         * `perform`. */
    private import ModelingMetadata::*;
    private import StandardViewDefinitions::*;

    // A requirement's ID is its short name, and its statement its `doc`. The
    // requirements nested in one are its subrequirements: satisfying it
    // satisfies them too. An ID says where its requirement sits: REQ-1.2 is
    // the second in REQ-1.

    // --- planning and following a survey -----------------------------------
    requirement <'REQ-1'> 'Survey planning' {
        doc /* The system shall let the operator plan a survey by marking its area
             * on a map, and shall fly no route the operator has not approved. */
        @Rationale { text = "The operator stays responsible for where the drone flies."; }

        requirement <'REQ-1.1'> 'Leg coverage' {
            doc /* The system shall cover a marked area with survey legs whose camera
                 * footprints overlap by at least 70 % from one leg to the next. */
            @Rationale { text = "Stitching the images into one map needs that much overlap."; }
        }
    }

    requirement <'REQ-2'> 'Progress display' {
        doc /* The system shall show the operator the drone's position, and how much
             * of the survey it has flown, at least once a second. */
    }

    requirement <'REQ-3'> 'Live video' {
        doc /* The system shall show the operator the camera's video at most 500 ms
             * after it is captured. */
        @Rationale { text = "The operator judges what the drone sees from the video; an older picture misleads."; }
    }

    // --- flying it ---------------------------------------------------------
    requirement <'REQ-4'> 'Automatic survey' {
        doc /* The drone shall fly an approved survey from take-off to landing with
             * no one at the controls. */

        requirement <'REQ-4.1'> 'Track accuracy' {
            doc /* The drone shall keep within 2 m of its planned track, in winds up
                 * to 10 m/s. */
            @Rationale { text = "Legs that drift apart leave gaps in the overlap the map needs."; }
        }
    }

    requirement <'REQ-5'> 'Endurance' {
        doc /* The drone shall fly for at least 30 min on one battery, carrying its
             * camera and both computers. */
        @Rationale { text = "A typical site is then surveyed in one flight, with no seam in the map where a battery was changed."; }
    }

    // --- when something goes wrong -----------------------------------------
    requirement <'REQ-6'> 'Safe flight' {
        doc /* The drone shall end its flight safely when an obstacle blocks its
             * route, its radio link is lost or its battery runs low. */

        requirement <'REQ-6.1'> 'Obstacle avoidance' {
            doc /* The drone shall detect an obstacle in its path at least 15 m ahead,
                 * and stop before it. */
            @Rationale { text = "Power lines, cranes and trees are not on the operator's map."; }

            requirement <'REQ-6.1.1'> 'Onboard detection' {
                doc /* The drone shall detect obstacles with its own computers, using
                     * neither the radio link nor the ground station. */
                @Rationale { text = "Detection on the ground would come too late, and stop when the link does."; }
            }
        }
        requirement <'REQ-6.2'> 'Lost link' {
            doc /* The drone shall return to its launch point and land when the radio
                 * link has been lost for 5 s. */
        }
        requirement <'REQ-6.3'> 'Low battery' {
            doc /* The drone shall return to its launch point once its battery holds
                 * only the charge to get there, with 20 % in reserve. */
        }
    }

    // --- the link and the record -------------------------------------------
    requirement <'REQ-7'> 'Link range' {
        doc /* The radio link shall carry the video and the telemetry up to 2 km from
             * the ground station, in line of sight. */
        @Rationale { text = "The operator can then stand at one edge of a site and reach its far side."; }
    }

    requirement <'REQ-8'> 'Link encryption' {
        doc /* The system shall encrypt and authenticate everything that crosses the
             * radio link. */
        @Rationale { text = "Video of private land must not be readable, nor commands to the drone forgeable, by anyone within radio range."; }
    }

    requirement <'REQ-9'> 'Flight record' {
        doc /* The drone shall record every flight's telemetry and health reports,
             * to be read after landing. */
        @Rationale { text = "An incident has to be traceable to its cause."; }
    }

    view 'Requirements — Survey Drone' : GridView {
        doc /* What the survey drone system shall do for an operator surveying an
             * area from the air. */
        expose SystemRequirements;
    }
}

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
             * on a map. */

        requirement <'REQ-1.1'> 'Leg coverage' {
            doc /* The system shall cover a marked area with survey legs whose camera
                 * footprints overlap by at least 70 % from one leg to the next. */
            @Rationale { text = "Stitching the images into one map needs that much overlap."; }
        }
    }

    requirement <'REQ-2'> 'Progress display' {
        doc /* While the drone flies a survey, the system shall show the operator
             * its position, and how much of the survey it has flown, at least once
             * a second. */
    }

    requirement <'REQ-3'> 'Live video' {
        doc /* While the drone flies, the system shall show the operator the
             * camera's video at most 500 ms after it is captured. */
        @Rationale { text = "The operator judges what the drone sees from the video; an older picture misleads."; }
    }

    // --- flying it ---------------------------------------------------------
    requirement <'REQ-4'> 'Automatic survey' {
        doc /* When the operator approves a survey, the drone shall fly it from
             * take-off to landing on its own. */
        @Rationale { text = "The operator stays responsible for where the drone flies."; }

        requirement <'REQ-4.1'> 'Track accuracy' {
            doc /* While it flies a survey in winds up to 10 m/s, the drone shall
                 * keep within 2 m of its planned track. */
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
        doc /* If an obstacle blocks its route, its radio link is lost or its
             * battery runs low, then the drone shall end its flight safely. */

        requirement <'REQ-6.1'> 'Obstacle avoidance' {
            doc /* If an obstacle is in its path, then the drone shall detect it at
                 * least 15 m ahead and stop before it. */
            @Rationale { text = "Power lines, cranes and trees are not on the operator's map."; }

            requirement <'REQ-6.1.1'> 'Onboard detection' {
                doc /* The drone shall detect obstacles with its own computers
                     * alone. */
                @Rationale { text = "Detection on the ground would come too late, and stop when the link does."; }
            }
        }
        requirement <'REQ-6.2'> 'Lost link' {
            doc /* If the radio link has been lost for 5 s, then the drone shall
                 * return to its launch point and land. */
        }
        requirement <'REQ-6.3'> 'Low battery' {
            doc /* If its battery holds only the charge to reach its launch point,
                 * with 20 % in reserve, then the drone shall return there. */
        }
    }

    // --- the link and the record -------------------------------------------
    requirement <'REQ-7'> 'Link range' {
        doc /* The system shall carry the video and the telemetry between the drone
             * and the ground station up to 2 km apart, in line of sight. */
        @Rationale { text = "The operator can then stand at one edge of a site and reach its far side."; }
    }

    requirement <'REQ-8'> 'Link encryption' {
        doc /* The system shall encrypt and authenticate everything that crosses the
             * radio link. */
        @Rationale { text = "Video of private land must not be readable, nor commands to the drone forgeable, by anyone within radio range."; }
    }

    requirement <'REQ-9'> 'Flight record' {
        doc /* While it flies, the drone shall record its telemetry and health
             * reports, to be read after landing. */
        @Rationale { text = "An incident has to be traceable to its cause."; }
    }

    view 'Requirements — Survey Drone' : GridView {
        doc /* What the survey drone system shall do for an operator surveying an
             * area from the air. */
        expose SystemRequirements;
    }
}

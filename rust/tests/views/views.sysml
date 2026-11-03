// Views written by hand, for what no model draws: `npm run oracle` records what the
// TypeScript makes of each, and the port is checked against it.
package ByHand {
    private import StandardViewDefinitions::*;
    private import ModelingMetadata::*;

    // A function tree whose root is wider than the stem over its one child sits
    // further right than the child's row would put it.
    package WideRoot {
        action def 'A function whose name is far wider than anything below it' {
            action narrow {
                action leaf;
            }
        }
    }

    // A function without a name is named after nothing, so it and the tree it is
    // in share a name: one box, and a branch to itself that takes no part in the
    // layout. Two without a name in one tree are one box with two parents.
    package Unnamed {
        action def {
            action;
        }
        action def Twice {
            action { action a; }
            action { action b; }
        }
    }

    // Use cases above the trees: one refined by two trees far apart, so neither
    // arrow meets it straight on, and one nothing refines, drawn last.
    package Refined {
        use case def Lonely;
        use case def Unrefined;
        use case def Survey;
        action def Left {
            action a1;
            action a2;
            action a3;
        }
        action def Middle {
            action m1 {
                action m11;
            }
            action m2 {
                action m21;
            }
        }
        action def Right {
            action r1;
        }
        #refinement dependency Left to Survey;
        #refinement dependency Right to Survey;
        #refinement dependency Middle to Unrefined;
        #refinement dependency Left to Middle;
    }

    // A typed action usage in a package is a use of a tree, not one of its own.
    package Used {
        action def Tree {
            action branch;
        }
        action used : Tree;
        action own {
            action part1;
        }
    }

    // Use cases that include themselves, include ones that are not their
    // neighbours, and have secondary actors.
    package Including {
        part def System;
        part def Operator;
        part def Bystander;
        use case def Operate {
            subject system : System;
            actor operator : Operator;
        }
        use case one : Operate {
            include one;
            include two;
            include three;
            include four;
        }
        use case two : Operate {
            include three;
            subject system;
            actor bystander : Bystander;
        }
        use case three : Operate;
        use case four : Operate {
            include one;
        }
    }

    // Messages with a payload and no name, with neither, and to and from ports.
    package Messages {
        part def Ping;
        port def Socket;
        part def Node {
            port socket : Socket;
        }
        occurrence def Exchange {
            part a : Node;
            part b : Node;
            // A port of a part, named on its own, stands for the part.
            part c {
                port plug : Socket;
                message plugged from plug to b;
            }
            message of Ping from a to b;
            message from b to a;
            message reply of Ping from a.socket to b.socket;
            message named from b.socket to a;
        }
    }

    // The functions without a name cannot be exposed by name: the views of each
    // package's elements that npm run oracle makes draw them.
    view 'Wide root' : GeneralView {
        expose WideRoot::'A function whose name is far wider than anything below it';
    }
    view 'Refined functions' : GeneralView {
        expose Refined::Lonely;
        expose Refined::Unrefined;
        expose Refined::Survey;
        expose Refined::Left;
        expose Refined::Middle;
        expose Refined::Right;
    }
    view 'A package and a tree' : GeneralView {
        expose Used;
        expose Used::Tree;
    }
    view 'Use cases including' : GeneralView {
        expose Including;
    }
    view 'Messages' : SequenceView {
        expose Messages::Exchange;
    }
}

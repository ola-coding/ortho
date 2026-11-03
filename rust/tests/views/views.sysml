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

    // Packages of software: an import of a package's members' members too, one of
    // a single element, one of a package twice, one of itself, and one reaching off
    // the view. A module's conjugated port, a port without a type, and a module
    // with a name far wider than its package's grid.
    package Software {
        package Api {
            port def Orders;
            part def 'A module whose name is far wider than its package' {
                port needs : ~Orders;
                port untyped;
            }
        }
        package Store {
            private import Api::**;
            private import Api::Orders;
            private import Store::*;
            private import Requirements::*;
            part def Table :> Api::'A module whose name is far wider than its package';
            part def Index;
            part def Log;
            part def Cache;
        }
        package Requirements {
            requirement def Fast;
        }
    }

    // Parts with multiplicities, a type that contains itself, ports inherited
    // through two supertypes that share one, connections between ports, to a part
    // without a port, to a port of a part that redefines its type's, back to the
    // same port, to a part the usage shadows, and without a name but typed.
    package Product {
        port def Plug;
        interface def Cable;
        part def Socketed {
            port plug : Plug;
        }
        part def Left :> Socketed;
        part def Right :> Socketed {
            port spare : Plug;
        }
        part def Both :> Left, Right;
        part def Nested {
            part inner : Nested [0..1];
        }
        part def Assembly {
            part a : Both [2];
            part b : Right [1..*];
            part n : Nested;
            part bare;
            interface : Cable connect a.plug to b.spare;
            connect a.spare to bare;
            connect b.plug to b.plug;
            connection link connect bare to n.inner;
        }
        part product : Assembly {
            // Shadows the type's, so the type's connection to it lands nowhere.
            part bare;
            part extra : Socketed;
            connect extra.plug to a.plug;
        }
    }

    // Hosts in devices fitted once, in a device fitted twice, in a device that
    // contains itself, and in a package; a program allocated to two hosts, twice
    // to one, and a host allocated to itself.
    package Deployed {
        part sw {
            part app;
            part tool;
        }
        part def Computer;
        part def Board {
            part cpu : Computer;
        }
        part def Rack {
            part board : Board;
            part host : Computer;
        }
        part def Twice {
            part host : Computer;
        }
        part def Loop {
            part again : Loop2;
            part host : Computer;
            allocate sw.tool to host;
        }
        part def Loop2 {
            part back : Loop;
        }
        part rack : Rack;
        part one : Twice;
        part two : Twice;
        part loose : Computer;
        allocation def Runs;
        allocate sw.app to rack.board.cpu;
        allocate sw.app to rack.host;
        allocate sw.app to rack.host;
        allocate sw.tool to one.host;
        allocate sw.tool to loose;
        allocate loose to loose;
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
    view 'Software' : GeneralView {
        expose Software::Api;
        expose Software::Store;
        expose Software::Requirements;
    }
    view 'Product' : InterconnectionView {
        expose Product::product;
    }
    view 'Deployed' : GeneralView {
        expose Deployed;
    }
}

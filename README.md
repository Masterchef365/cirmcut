# Circuit simulator (WIP)
This circuit simulator uses the Sparse Tableau Analysis (STA) scheme to construct a linear system for the circuit at a given moment. That system can either be solved directly, or, for nonlinear circuits can be used to iterate solutions using the Newton-Raphson algorithm. 

## What works
* Linear solver (only solves linear circuits, but is very fast)
* Newton-Raphson solver (for nonlinear circuits)
* Wires, Resistors, Capacitors, Inductors, Voltage sources, Current sources, Switches

## What kinda works
* Diodes, Transistors. These use oversimplified, probably buggy models at the moment.

## TODO
- [ ] Different diode equation? NR solver requires damping ...
- [ ] Transistor allows more current than it should
- [ ] Fix hitboxes in general. Sometimes drag handles are overlapped by components.
- [ ] Any amount of test framework whatsoever
- [ ] Make a popup for selected components, to edit their parameters
- [ ] Component connecting to ground 

References:
* Hachtel, Gary, R. Brayton, and Fred Gustavson. "The sparse tableau approach to network analysis and design." IEEE Transactions on circuit theory 18.1 (1971): 101-113.
* Techniques for circuit simulation M. B. Patil www.ee.iitb.ac.in/~sequel Department of Electrical Engineering Indian Institute of Technology Bombay
* Computer Methods for Circuit Analysis and Design Jiri Vlach

A client is whoever sends requests to your system: a browser, a mobile app, or another company's server calling your API. In a design it is where every request starts, and it never does any work of its own.

In the simulator, requests leave the client at random moments, like real users, and each one follows the wires to a component that can serve it. If no answer comes back within a few seconds, the client gives up and the request counts as an error.

In the Sandbox the client's **rps** sets the load. In a problem the scenario sets the total load, and several client cards share it in proportion to their rps.

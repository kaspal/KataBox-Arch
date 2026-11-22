A saga runs an operation as a sequence of local transactions, one per service: reserve the stock, charge the card, book the courier. Each step commits on its own. If a later step fails, the saga runs compensating actions for the steps already done, in reverse order: refund the card, release the stock.

An orchestrator can drive the steps, calling each service and recording progress in its own table, so it can pick up where it left off after a crash. The other style is choreography, where each service reacts to the previous one's event; it has no central coordinator, but the flow is harder to follow.

A compensation is a new action, not a rollback. The customer may have seen the charge before the refund arrives. Design the steps so that this in-between state is acceptable, and put the step most likely to fail early.

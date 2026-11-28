A config service holds the settings and **feature flags** your apps read: timeouts, limits, which features are on. Changing a value there changes how the apps behave without a new build or deploy. Apps get changes by **stream**, pushed as soon as they are made, or by **poll**, asking every few seconds.

That power is also the risk. A config push is a deploy in disguise, and a bad value pushed to everyone at once takes the whole fleet down together. A **staged** rollout sends the change to one group of servers at a time, waits a **bake time** to watch the error rate, and halts if errors rise. A **kill switch** goes one step further and rolls the change back within a second once the rollout halts. The simulator models both.

The config service is not on the request path. Each app keeps a local copy of its config and reads from it, so when the service is down, requests keep working with the last known values and only new changes wait.

**How the simulator bills this.** LaunchDarkly, AWS AppConfig and Unleash all cost a flat $20 a month. That figure is a model, not a list price: these products don't sell an instance (AWS AppConfig, for one, bills $0.20 per million requests), and the simulator counts neither requests nor flags. The price doesn't grow with the number of apps, flags or changes.

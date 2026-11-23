An identity provider handles sign-in for you. The user proves who they are to the provider, and the provider hands back a signed **token**. From then on, each request carries that token, and your system checks it before doing any work. Passwords, MFA and account recovery stay with the provider.

There are two very different operations here. A **login** is rare and slow, and the provider limits how many it accepts per second. A **token check** happens on every request. If the app asks the provider each time (remote verification), every request waits on the provider and fails when it is down. If the app checks the token's signature itself with the provider's cached signing keys (local verification), the check is almost free and keeps working through an outage.

So the usual design is: send logins to the provider, verify tokens locally, and make sure a login surge stays under the provider's rate limit.

**How the simulator bills this.** Auth0, Cognito and Okta bill $5 per million logins, a stand-in for their price per monthly active user (teaching price; source to be confirmed). Keycloak is self-hosted at $70 a month per server, with no fee per login. Local token checks cost nothing.

# Hotel Booking

## The product

You run a hotel booking site. Travellers search for hotels in a city for some dates, look through the results, and book a room. Searching is most of the traffic, and the same few searches (a big city on a holiday weekend) are made over and over. Booking is rare by comparison, but it must be right: a room-night can be sold only once. If two travellers book the same room for the same night, one of them turns up to find it taken, so a double booking must never happen. A search result may be a minute old; the booking is what decides.

## What you must meet

| Goal | Target |
| --- | --- |
| Peak traffic | 5,000 rps |
| p95 latency | 200 ms or less |
| Availability | 99.9% or more |
| Error rate | 1% or less |
| Budget | $1,800 per month |

## The traffic

Travellers' requests reach your App Servers, through an edge if you want one.

- **Search hotels for some dates** (98%). An App Server runs the search, through a cache if you add one, on a search engine. There are 200,000 different searches, and a few popular city-and-date searches get a large share of them.
- **Book a room** (2%). An App Server reads the room's availability from the SQL database, then writes the booking. 20% of bookings are for one of 10 rooms that many people want at once. If two bookings both read "free" before either writes, both succeed, and the room is sold twice. The simulator counts that as a conflict.

## The tests

One test is visible. You can run it as often as you like:

- **A normal week**: the traffic above, at peak.

Two more tests are hidden, and you see only their names: **Holiday bookings open** and **Everyone wants the last room**. Your design has to pass all three.

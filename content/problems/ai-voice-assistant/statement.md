# Voice Assistant

## The product

You run the voice assistant inside a cooking app. People talk to it with their hands full: "what's next?", "how long do the onions take?". The app keeps a WebSocket session open to your backend while it is on screen. When the user stops talking, the app sends the clip over that session, and your backend answers in three steps: a speech-to-text model writes down what was said, a language model writes a short reply, and a text-to-speech model turns the reply into audio, which plays on the phone. In a conversation, a pause longer than about a second feels broken, so what matters is the time from the end of the user's words to the first sound of the reply.

## What you must meet

| Goal | Target |
| --- | --- |
| Peak traffic | 20 turns/s |
| p95 latency (to the first sound of the reply) | 1,200 ms or less |
| Sessions connected | 95% or more, at all times |
| Availability | 99.9% or more |
| Errors | 1% or less |
| Budget | $10,000 per month |

## The traffic

- **Devices with a session open**. 150,000 phones keep a WebSocket session open on your WebSocket Gateway cards. Most of the time they are silent.
- **A spoken turn** (100% of the turns). The turn travels over the session to a WebSocket Gateway, which passes it to your App Servers. The App Server calls three Model Inference cards in order, and keeps its slot until the third one answers:
  1. **Speech to text.** Set this card's Model type to "Embeddings": in the simulator that is a model that reads its whole input and writes nothing it counts as output. Its time is its Time to first token plus the input divided by its Prompt speed.
  2. **The language model.** Model type "Chat (text generation)". It reads about 300 tokens (the recipe step, the conversation so far and the transcript; 450 at p95) and writes a reply of about 15 tokens (30 at p95): a spoken answer is one short sentence. Its time is its Time to first token, plus the prompt divided by its Prompt speed, plus the reply divided by its Output speed.
  3. **Text to speech.** Model type "Classifier": in the simulator it writes one unit of output, which stands for the first chunk of audio. The user hears the reply from that moment. Set its Prompt speed to 50,000 tokens/s, because it reads only the reply, not the whole prompt.

The simulator draws one prompt size per turn and charges it to all three model calls; at the speech-to-text card it stands for the length of the clip.

You can host the models yourself (vLLM on GPU for the language model, NVIDIA Triton for the speech models) or call a hosted model API. Keep each card's Time to first token, Output speed and Prompt speed at the card's defaults, except the Prompt speed of the text-to-speech card above: they stand for the model you pick, not for a dial you can turn. On a self-hosted GPU, a vLLM card runs 16 requests at a time per GPU (Batch slots per GPU), and in the simulator each request writes its tokens more slowly the more slots are busy: at the Output speed × (1 − 0.5 × busy slots ÷ all slots). A hosted API has no slots to fill and bills every token, and its account allows 4,000 requests and 400,000 tokens a minute per card unless you raise the limits.

The app gives up on a turn after 5 s (the Client's default Timeout).

## The tests

One test is visible. You can run it as often as you like:

- **A normal evening**: the traffic above, at peak.

Two more tests are hidden, and you see only their names: **One model slows down by 300 ms** and **Three times the turns for 30 s**. Your design has to pass all three.

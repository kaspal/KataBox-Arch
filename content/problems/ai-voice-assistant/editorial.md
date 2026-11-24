# Editorial

## Where the load goes

A turn is three model calls in a row, and the App Server waits for each one before it starts the next. The user hears the sum. With the card defaults and a typical turn (a 300-token prompt and a 15-token reply), the stages take:

- speech to text on Triton: 5 ms + 300 ÷ 5,000 tokens/s = 65 ms;
- the language model on vLLM: 200 ms to start, 300 ÷ 5,000 = 60 ms to read the prompt, and 15 ÷ 80 tokens/s ≈ 188 ms to write the reply, about 450 ms in all;
- text to speech on Triton: 5 ms + 300 ÷ 50,000 = 6 ms, plus one unit of output, about 24 ms.

That is about 540 ms. A long turn (450 tokens in, 30 out) takes about 95 + 665 + 26 ≈ 790 ms. Most of it is the language model, and most of that is writing the reply.

Two things eat the rest of the 1.2 s. The first is the start-up time: the hosted APIs' Time to first token is 500 to 600 ms by default, against vLLM's 200 ms, so a hosted language model spends about half the budget before it writes a word. The second is the GPU itself. A vLLM GPU has 16 slots, and each request writes at 80 × (1 − 0.5 × busy ÷ slots) tokens a second. At 20 turns a second, with each language model call taking about 0.5 to 0.6 s, about 12 slots are busy at any moment. On one GPU that is three quarters of its slots, so every reply is written at about 50 tokens a second instead of 80, and the long reply takes 600 ms instead of 375 ms.

The hidden tests take the spare time away. "One model slows down by 300 ms" adds 300 ms to every call of one Model Inference card (the first one placed in the design; in the designs here, the language model) from 15 seconds in, for 40 seconds. "Three times the turns for 30 s" sends 60 turns a second from second 15 to second 45: about 30 busy slots.

## The reference design

A Mobile client holds its session on a Socket.IO WebSocket Gateway with 2 instances, which holds 200,000 connections at its defaults (100,000 each), more than the 150,000 sessions. The gateway passes turns to 2 Go App Servers. They call an NVIDIA Triton with the Model type "Embeddings" for speech to text, a vLLM on GPU with 4 instances for the language model, and an NVIDIA Triton with the Model type "Classifier" and a Prompt speed of 50,000 tokens/s for text to speech. This costs about $6,152 per month: $2,936 for the 4 language model GPUs, $1,468 for each Triton card (2 GPUs each), $140 for the App Servers and $140 for the gateway.

On a normal evening the 64 language model slots are about 15% busy, so replies are written at close to the full 80 tokens a second. The p95 is about 818 ms, with no errors and every session connected.

When the language model slows down by 300 ms, each call holds its slot 0.3 s longer, about 6 more busy slots: still under a third of 64. The p95 rises to about 1,079 ms, under the limit, with no errors. If the slowdown hit one of the speech cards instead, it would add the same 300 ms and nothing else.

When the turns triple, about 30 of the 64 slots are busy, and replies are written at about three quarters of the full speed. The p95 is about 851 ms, with no errors.

For the cost challenge, a proof design comes in at $4,614, 75% of the reference's cost, at the 75% target. It keeps the same shape with 3 language model GPUs instead of 4, one Triton instance for speech to text and one App Server instance.

## Why the first try breaks

A common first try is the same design with one language model GPU: the average load fits in its 16 slots, and it is only about three quarters busy. It costs about $3,950 and passes "A normal evening" with a p95 of about 1,145 ms. It passes only just: in 15 of the 60 seconds the p95 of that second is over 1.2 s, because every reply is written at about 50 tokens a second.

When the language model slows down by 300 ms, 6 more slots are busy, and 12 + 6 is more than 16. Turns start to queue for a slot. The p95 climbs second by second, from about 1.9 s at second 19 to about 4.8 s at second 38, and from second 41 the app gives up on turns after 5 s. About 29.8% of the turns fail.

When the turns triple, about 30 slots are wanted and 16 exist. The queue grows from the first second, and from second 22 most turns time out. About 73.5% of the turns fail.

Moving the language model to a hosted API removes the slots but not the problem. With a Gemini API card (500 ms to the first token, 120 tokens a second) and its limits raised, a normal evening has a p95 of about 939 ms, and the 300 ms slowdown takes it to about 1,215 ms, over the limit. It also costs about $31,500 a month, because every token is billed: a Claude API card is slower (p95 about 1,168 ms) and costs about $63,300. Two GPUs pass a normal evening (about 868 ms) and the slowdown (about 1,168 ms, only just), and still fail the tripled turns with about 31.6% errors.

The point is this: a voice turn is a chain, and its budget is the sum of every stage, including the time each one waits to start and the time it spends on a busy GPU. Spend as little of it as you can on each stage (models that start fast, short replies, the first chunk of audio instead of the whole clip), and keep the GPUs mostly idle, so a slow stage or a crowd has spare time to use.

## What the simulator leaves out

Streaming between the stages is not simulated. Here each stage waits for the one before it to finish: the language model writes the whole reply before text to speech starts. A real voice pipeline streams: speech to text transcribes while the user is still talking, the language model's first words go to text to speech as soon as they are written, and the reply plays while the rest is written. With a 30-token reply and text to speech starting after the first 6 tokens, that saves about (30 − 6) ÷ 80 ≈ 300 ms. This problem stands in for it with what the simulator does model: short replies, text to speech counted to its first chunk of audio, models that start fast, and GPU headroom. Its turn times are those of a pipeline that hands over finished results, so a streamed pipeline would be faster than shown.

The quality of the transcripts, replies and voices is not simulated, and nothing here says which model sounds or answers better. The speech models are Model Inference cards with the "Embeddings" and "Classifier" types, which stand for their timing only: the simulator draws one prompt size per turn and charges it to all three calls. The audio itself, the network from the phone, detecting the end of speech, users interrupting the reply, and GPU memory are not simulated. The time to first token is recorded at each model call, so this problem judges the turn's latency instead. The GPU price is a g5.xlarge-class figure, and the token prices are the registry's teaching figures.

## Things to try

- Set the language model to 1 GPU and run "A normal evening". Look at the seconds whose p95 is over 1.2 s, and work out the reply speed at 12 busy slots of 16.
- Replace the vLLM card with a Gemini API card and raise its limits. Compare the p95 and the monthly cost with the reference.
- Set the language model to 2 GPUs and run "Three times the turns for 30 s". Count the busy slots it needs.

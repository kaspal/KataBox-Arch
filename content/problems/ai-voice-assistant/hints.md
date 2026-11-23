## 1

A turn waits for three models, one after the other, so the user hears the sum of their times. Add them up for a typical turn and for a long one. How much of the 1.2 s is left over? And what happens to that spare time when one stage gets slower, or when three times as many people talk at once?

## 2

Pick models whose fixed start-up time is small: the hosted APIs' first token takes 500 to 600 ms, which is half the budget before a single word is written. A self-hosted language model starts faster, but on a GPU every busy slot slows the others down, and a full GPU queues turns. Size the GPUs so they stay mostly idle at the peak and at three times the peak; that idle time is the spare time your turn budget needs.

## 3

A Mobile client connects over a WebSocket edge to a Socket.IO WebSocket Gateway (2 instances), which passes turns to 2 Go App Servers. The App Servers call three Model Inference cards: an NVIDIA Triton with the Model type "Embeddings" for speech to text, a vLLM on GPU with 4 instances for the language model, and an NVIDIA Triton with the Model type "Classifier" and a Prompt speed of 50,000 tokens/s for text to speech.

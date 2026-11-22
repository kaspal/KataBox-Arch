import { describe, expect, it, vi } from "vitest";
import { screen, waitFor, within } from "@testing-library/react";
import { setFakeApi, useFakeApi } from "@/test/mockApi";
import { renderApp } from "@/test/renderApp";
import { unnamed } from "@/test/a11y";

vi.mock("@/lib/api", async () => (await import("@/test/mockApi")).apiModule);

const fake = useFakeApi();

describe("quizzes", () => {
  it("a Guide entry ends with three questions; answering shows the right option and why", async () => {
    const { user } = renderApp("/guide/cache");
    const quiz = await screen.findByTestId("quiz");
    const questions = within(quiz).getAllByTestId("quiz-question");
    expect(questions).toHaveLength(3);
    // A wrong answer: the right option is marked, the explanation shows, and it comes back.
    await user.click(within(questions[0]!).getByRole("button", { name: "Stronger consistency" }));
    const feedback = await within(questions[0]!).findByTestId("quiz-feedback");
    expect(feedback).toHaveTextContent("Not quite.");
    expect(feedback).toHaveTextContent("Hits are served from memory.");
    expect(feedback).toHaveTextContent("Back for review on");
    expect(within(questions[0]!).getByRole("button", { name: /Fewer database reads/ })).toBeDisabled();
    expect(fake().api.answerQuiz).toHaveBeenCalledWith("guide:cache:1", 0, expect.stringMatching(/^\d{4}-\d{2}-\d{2}$/));
    // A right answer: no review date.
    await user.click(within(questions[1]!).getByRole("button", { name: "How stale a copy can be" }));
    const right = await within(questions[1]!).findByTestId("quiz-feedback");
    expect(right).toHaveTextContent("Correct.");
    expect(right).not.toHaveTextContent("Back for review");
    expect(unnamed(document.body)).toEqual([]);
  });

  it("Home shows what is due, and the review page goes through it one question at a time", async () => {
    setFakeApi({ due: ["guide:cache:1", "pattern:saga:3"], settings: { "tour.offered": true } });
    const { user } = renderApp("/");
    const card = await screen.findByTestId("home-review");
    expect(card).toHaveTextContent("2 questions");
    await user.click(card);
    expect(await screen.findByTestId("review-position")).toHaveTextContent("Question 1 of 2 · Cache");
    await user.click(screen.getByRole("button", { name: "Fewer database reads" }));
    await user.click(await screen.findByRole("button", { name: "Next question" }));
    expect(screen.getByTestId("review-position")).toHaveTextContent("Question 2 of 2 · Pattern: saga");
    await user.click(screen.getByRole("button", { name: "Every read goes to the database for a while" }));
    await user.click(await screen.findByRole("button", { name: "Finish" }));
    await waitFor(() => expect(screen.getByRole("heading", { name: "Review done" })).toBeInTheDocument());
  });

  it("an empty queue says so", async () => {
    renderApp("/review");
    expect(await screen.findByRole("heading", { name: "Nothing to review today" })).toBeInTheDocument();
  });
});

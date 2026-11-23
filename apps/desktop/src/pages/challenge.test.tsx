// M8.5 and M8.6: problem tracks, the "same SLO, half the cost" badge, filter and banner, and the
// Progress page.
import { describe, expect, it, vi } from "vitest";
import { screen, waitFor, within } from "@testing-library/react";
import type { ProblemBrief, ProblemSummary } from "@/bindings";
import { useFakeApi } from "@/test/mockApi";
import { renderApp } from "@/test/renderApp";
import { FAKE_BRIEF, fakeJudgement } from "@/test/fakeProblems";
import { unnamed } from "@/test/a11y";

vi.mock("@/lib/api", async () => (await import("@/test/mockApi")).apiModule);
const fake = useFakeApi();

const challenge = { ratio: 0.5, target_cost: 600, best_cost: null, solved: false, attempts: 0 };

function summaries(): ProblemSummary[] {
  return [
    { ...FAKE_BRIEF.summary, challenge },
    { ...FAKE_BRIEF.summary, id: "pastebin", number: 2, title_en: "Pastebin", title_es: "Pastebin", challenge: { ...challenge, ratio: 0.6 } },
    { ...FAKE_BRIEF.summary, id: "typeahead", number: 10, title_en: "Typeahead", title_es: "Autocompletado" },
    { ...FAKE_BRIEF.summary, id: "ai-chatbot", number: 1, title_en: "AI Chatbot", title_es: "Chatbot de IA", track: "ai", status: "solved" },
  ];
}

describe("tracks and the cost challenge on Problems", () => {
  it("shows the tracks with their progress, numbers AI problems, and filters challenges", async () => {
    vi.mocked(fake().api.listProblems).mockResolvedValue(summaries());
    const { user } = renderApp("/problems");
    await screen.findAllByTestId("problem-row");
    expect(screen.getAllByTestId("problem-row")).toHaveLength(3);
    const badges = screen.getAllByTestId("challenge-badge");
    expect(badges[0]).toHaveTextContent("Half the cost");
    expect(badges[1]).toHaveTextContent("−40% cost");
    await user.click(screen.getByTestId("challenge-filter"));
    expect(screen.getAllByTestId("problem-row")).toHaveLength(2);
    await user.click(screen.getByTestId("track-ai"));
    expect(screen.getByTestId("track-ai")).toHaveTextContent("1/1");
    await user.click(screen.getByTestId("challenge-filter"));
    const ai = screen.getAllByTestId("problem-row");
    expect(ai).toHaveLength(1);
    expect(ai[0]).toHaveTextContent("AI 1");
    expect(unnamed(document.body)).toEqual([]);
  });
});

describe("the challenge banner", () => {
  const brief = (solved: boolean): ProblemBrief => ({
    ...FAKE_BRIEF,
    solved,
    summary: { ...FAKE_BRIEF.summary, status: solved ? "solved" : null, challenge },
  });

  it("is locked until the problem is solved", async () => {
    vi.mocked(fake().api.problemBrief).mockResolvedValue(brief(false));
    renderApp("/problems/url-shortener");
    const banner = await screen.findByTestId("challenge-banner");
    expect(banner).toHaveTextContent("Solve the problem to open its cost challenge.");
    expect(within(banner).queryByTestId("challenge-submit")).not.toBeInTheDocument();
  });

  it("then judges the canvas against the target", async () => {
    vi.mocked(fake().api.problemBrief).mockResolvedValue(brief(true));
    vi.mocked(fake().api.submitChallenge).mockResolvedValue({ ...fakeJudgement("url-shortener", "accepted"), cost_month: 540 });
    const { user } = renderApp("/problems/url-shortener");
    const banner = await screen.findByTestId("challenge-banner");
    expect(banner).toHaveTextContent("$600 a month (50% of the reference's cost)");
    await user.click(within(banner).getByTestId("challenge-submit"));
    await waitFor(() => expect(within(banner).getByRole("status")).toHaveTextContent("Challenge passed at $540 (target $600)."));
    expect(fake().api.submitChallenge).toHaveBeenCalledWith("url-shortener", expect.objectContaining({ schema_version: 4 }));
  });
});

describe("Progress", () => {
  it("counts per track and the challenges", async () => {
    vi.mocked(fake().api.listProblems).mockResolvedValue(summaries());
    vi.mocked(fake().api.progress).mockResolvedValue({
      easy: { total: 4, attempted: 0, solved: 1 },
      medium: { total: 0, attempted: 0, solved: 0 },
      hard: { total: 0, attempted: 0, solved: 0 },
      streak: 2,
      recommended: null,
      continue_with: null,
      challenges_solved: 1,
      challenges_total: 2,
    });
    renderApp("/progress");
    const tracks = await screen.findByTestId("progress-tracks");
    expect(tracks).toHaveTextContent("Core");
    expect(tracks).toHaveTextContent("AI systems");
    expect(screen.getByTestId("progress-challenges")).toHaveTextContent("Cost challenges: 1 of 2");
    expect(screen.getByTestId("progress-streak")).toHaveTextContent("2-day streak");
  });
});

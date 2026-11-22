import { afterEach, describe, expect, it } from "vitest";
import { act, render, screen } from "@testing-library/react";
import { REGISTRY } from "@/test/fakeApi";
import { ONE } from "@/test/fakeSamples";
import { useTelemetry } from "@/solve/run/telemetry";
import { DesignView, fitBox } from "@/solve/compare/DesignView";
import "@/i18n";

afterEach(() => useTelemetry.setState({ byId: {}, edgeRates: {} }));

describe("the read-only design view", () => {
  it("shows live numbers by node id, except in a still picture", () => {
    act(() => useTelemetry.setState({ byId: { app: { util: 54, queue: 0, down: false, overloaded: false } }, edgeRates: {} }));
    const vp = { x: 0, y: 0, zoom: 1 };
    const { unmount } = render(
      <div style={{ width: 800, height: 600 }}>
        <DesignView design={ONE} registry={REGISTRY} viewport={vp} label="live" />
      </div>,
    );
    expect(screen.getAllByTestId("component-card")[1]).toHaveTextContent("54% busy");
    unmount();
    render(
      <div style={{ width: 800, height: 600 }}>
        <DesignView design={ONE} registry={REGISTRY} viewport={vp} label="still" still />
      </div>,
    );
    expect(screen.getAllByTestId("component-card")[1]).not.toHaveTextContent("busy");
  });

  it("fits a box with a zoom floor callers can lower (thumbnails)", () => {
    const tall = { x: 0, y: 0, w: 200, h: 1200 };
    expect(fitBox(tall, { w: 200, h: 160 }, 12).zoom).toBe(0.3);
    expect(fitBox(tall, { w: 200, h: 160 }, 12, 0.1).zoom).toBeCloseTo((160 - 24) / 1200);
  });
});

import { describe, expect, it } from "vitest";
import { dragBox, fitBox, grabAt, isShareId } from "./locate.ts";

describe("mountain box", () => {
	it("fits the skyline with sky above and a little below", () => {
		const [x0, y0, x1, y1] = fitBox(
			[
				[100, 500],
				[900, 400],
			],
			1000,
			1000,
		);
		expect(x0).toBe(90);
		expect(x1).toBe(910);
		expect(y0).toBe(340); // 400 − max(60, 0.6 × 100)
		expect(y1).toBe(550); // 500 + max(50, 0.4 × 100)
	});

	it("knows edges, corners, the inside and the outside", () => {
		const box: [number, number, number, number] = [100, 100, 500, 300];
		expect(grabAt(box, [100, 200], 10)).toBe("w");
		expect(grabAt(box, [505, 295], 10)).toBe("se");
		expect(grabAt(box, [300, 200], 10)).toBe("move");
		expect(grabAt(box, [700, 200], 10)).toBeNull();
	});

	it("pulls an edge in, and keeps a moved box on the photo", () => {
		const box: [number, number, number, number] = [100, 100, 500, 300];
		expect(dragBox(box, "w", [100, 200], [250, 200], 1000, 1000)).toEqual([
			250, 100, 500, 300,
		]);
		expect(dragBox(box, "w", [100, 200], [490, 200], 1000, 1000)).toEqual([
			468, 100, 500, 300,
		]);
		expect(dragBox(box, "move", [300, 200], [1300, 200], 1000, 1000)).toEqual([
			600, 100, 1000, 300,
		]);
	});
});

describe("share ids", () => {
	it("accepts a UUID and nothing else", () => {
		expect(isShareId(crypto.randomUUID())).toBe(true);
		expect(isShareId("../photos/x")).toBe(false);
	});
});

describe("skyline shape", () => {
	it("tells a ridge from a straight smooth line", async () => {
		const { shapeOf, MIN_SHAPE } = await import("./locate.ts");
		const ramp = Array.from({ length: 100 }, (_, i): [number, number] => [
			i * 30,
			900 - i * 2,
		]);
		const ridge = Array.from({ length: 100 }, (_, i): [number, number] => [
			i * 30,
			900 + 60 * Math.sin(i / 5),
		]);
		expect(shapeOf(ramp, 3000)).toBeLessThan(MIN_SHAPE);
		expect(shapeOf(ridge, 3000)).toBeGreaterThan(MIN_SHAPE);
	});
});

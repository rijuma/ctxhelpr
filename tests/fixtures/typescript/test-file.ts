import { add } from "./simple";

describe("add function", () => {
  test("adds two positive numbers", () => {
    const result = add(1, 2);
    expect(result).toBe(3);
  });

  test("adds negative numbers", () => {
    const result = add(-1, -2);
    expect(result).toBe(-3);
  });

  describe("edge cases", () => {
    it("handles zero", () => {
      expect(add(0, 0)).toBe(0);
    });
  });
});

describe("multiply", () => {
  const helper = (a: number, b: number) => a * b;

  test("multiplies numbers", () => {
    expect(helper(2, 3)).toBe(6);
  });
});

/** Formats a number as currency */
export const formatCurrency = (amount: number, currency: string = "USD"): string => {
  return new Intl.NumberFormat("en-US", { style: "currency", currency }).format(amount);
};

export const identity = <T>(value: T): T => value;

export const pipe = (...fns: Function[]) => (x: unknown) => fns.reduce((v, f) => f(v), x);

const double = (n: number): number => n * 2;
const increment = (n: number): number => n + 1;

export const processNumber = pipe(double, increment);

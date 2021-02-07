export function hello(): string;
export function init_app(callback: (arg: string) => void);
export function app_command(
  arg: string,
  callback: (err: string | null, result: string) => void
);
export function app_command_promise(arg: string): Promise<string>;

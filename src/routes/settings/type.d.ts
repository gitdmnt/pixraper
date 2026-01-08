import { Temporal } from "@js-temporal/polyfill";

export interface Config {
  cookies: string;
  output: string;
  scraping_interval_millis: number;
}

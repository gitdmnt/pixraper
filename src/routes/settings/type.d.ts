export interface CookieProfile {
  id: string;
  name: string;
  cookies: string;
  is_valid: boolean | null;
}

export interface Config {
  cookies: string | null;
  output: string | null;
  scraping_interval_millis: number;
  cookie_profiles: CookieProfile[];
  active_profile_id: string | null;
}

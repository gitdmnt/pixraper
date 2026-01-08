interface CsvRow {
  id: number;
  title: string;
  isXRestricted: boolean;
  tags: string[];
  userId: number;
  createDate: string;
  generatedByAI: boolean;
  width: number;
  height: number;
  bookmarkCount: number;
  viewCount: number;
}
export type { CsvRow };

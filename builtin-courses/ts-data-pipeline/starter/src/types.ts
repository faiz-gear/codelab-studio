export interface UserRecord {
  name: string;
  age: number;
}

export interface RawEvent {
  id: string;
  category: string;
  amount: number;
  latencyMs: number;
  timestamp: number;
  tags: string[];
}

export interface NormalizedEvent {
  id: string;
  category: string;
  amount: number;
  latencyMs: number;
  timestamp: number;
  tags: string[];
}

export interface CategorySummary {
  count: number;
  totalAmount: number;
  maxLatencyMs: number;
}

export interface Page<T> {
  items: T[];
  nextCursor: string | null;
  hasMore: boolean;
}

export interface BatchResult<R> {
  fulfilled: R[];
  rejected: string[];
}

export interface PipelineSummary {
  processedCount: number;
  categories: Record<string, CategorySummary>;
  topCategory: string | null;
}

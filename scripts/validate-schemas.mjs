#!/usr/bin/env node
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const repoRoot = path.resolve(__dirname, '..');
const schemasDir = path.join(repoRoot, 'schemas');

const REQUIRED_SCHEMAS = [
  'codelab-pack.v1.schema.json',
  'codelab-checks.v1.schema.json',
  'workspace-meta.v1.schema.json',
  'workspace-progress.v1.schema.json',
  'workspace-metrics.v1.schema.json',
  'workspace-chat.v1.schema.json',
];

function deepEqual(a, b) {
  if (a === b) return true;
  if (typeof a !== typeof b || a === null || b === null) return false;
  if (Array.isArray(a) || Array.isArray(b)) {
    if (!Array.isArray(a) || !Array.isArray(b) || a.length !== b.length) return false;
    return a.every((item, idx) => deepEqual(item, b[idx]));
  }
  if (typeof a === 'object') {
    const keysA = Object.keys(a).sort();
    const keysB = Object.keys(b).sort();
    if (!deepEqual(keysA, keysB)) return false;
    return keysA.every((k) => deepEqual(a[k], b[k]));
  }
  return false;
}

function resolveRef(rootSchema, ref) {
  if (typeof ref !== 'string' || !ref.startsWith('#/')) {
    throw new Error(`Unsupported $ref: ${ref}`);
  }
  const parts = ref
    .slice(2)
    .split('/')
    .map((p) => p.replace(/~1/g, '/').replace(/~0/g, '~'));
  let cur = rootSchema;
  for (const part of parts) {
    if (cur === null || typeof cur !== 'object' || !(part in cur)) {
      throw new Error(`Unresolved $ref pointer: ${ref}`);
    }
    cur = cur[part];
  }
  return cur;
}

function matchesType(value, expectedType) {
  switch (expectedType) {
    case 'null':
      return value === null;
    case 'boolean':
      return typeof value === 'boolean';
    case 'string':
      return typeof value === 'string';
    case 'number':
      return typeof value === 'number' && Number.isFinite(value);
    case 'integer':
      return typeof value === 'number' && Number.isInteger(value);
    case 'array':
      return Array.isArray(value);
    case 'object':
      return value !== null && typeof value === 'object' && !Array.isArray(value);
    default:
      return true;
  }
}

function validateNode(rootSchema, schema, value, pointer) {
  if (schema === true) return [];
  if (schema === false) return [`${pointer}: value disallowed by false schema`];
  if (schema === null || typeof schema !== 'object') return [];

  const errors = [];

  if (typeof schema.$ref === 'string') {
    const target = resolveRef(rootSchema, schema.$ref);
    errors.push(...validateNode(rootSchema, target, value, pointer));
  }

  if (Array.isArray(schema.allOf)) {
    for (const sub of schema.allOf) {
      errors.push(...validateNode(rootSchema, sub, value, pointer));
    }
  }

  if (Array.isArray(schema.anyOf)) {
    const branchErrors = schema.anyOf.map((sub) => validateNode(rootSchema, sub, value, pointer));
    if (!branchErrors.some((errs) => errs.length === 0)) {
      const sample = branchErrors[0]?.[0] ?? 'no anyOf branch matched';
      errors.push(`${pointer}: value did not match anyOf (${sample})`);
    }
  }

  if (Array.isArray(schema.oneOf)) {
    const branchErrors = schema.oneOf.map((sub) => validateNode(rootSchema, sub, value, pointer));
    const matches = branchErrors.filter((errs) => errs.length === 0).length;
    if (matches !== 1) {
      const detail =
        matches === 0
          ? `matched 0 oneOf branches (${branchErrors[0]?.[0] ?? 'mismatch'})`
          : `matched ${matches} oneOf branches (expected 1)`;
      errors.push(`${pointer}: ${detail}`);
    }
  }

  if (schema.not !== undefined) {
    const notErrs = validateNode(rootSchema, schema.not, value, pointer);
    if (notErrs.length === 0) {
      errors.push(`${pointer}: value matched forbidden 'not' schema`);
    }
  }

  if (schema.type !== undefined) {
    const allowed = Array.isArray(schema.type) ? schema.type : [schema.type];
    if (!allowed.some((t) => matchesType(value, t))) {
      errors.push(
        `${pointer}: expected type ${allowed.join(' | ')}, got ${value === null ? 'null' : Array.isArray(value) ? 'array' : typeof value}`,
      );
      return errors;
    }
  }

  if ('const' in schema && !deepEqual(value, schema.const)) {
    errors.push(
      `${pointer}: expected const ${JSON.stringify(schema.const)}, got ${JSON.stringify(value)}`,
    );
  }

  if (Array.isArray(schema.enum)) {
    if (!schema.enum.some((allowed) => deepEqual(value, allowed))) {
      errors.push(
        `${pointer}: value ${JSON.stringify(value)} not in enum [${schema.enum.map((v) => JSON.stringify(v)).join(', ')}]`,
      );
    }
  }

  if (typeof value === 'string') {
    if (typeof schema.minLength === 'number' && value.length < schema.minLength) {
      errors.push(`${pointer}: string length ${value.length} < minLength ${schema.minLength}`);
    }
    if (typeof schema.pattern === 'string') {
      const re = new RegExp(schema.pattern);
      if (!re.test(value)) {
        errors.push(`${pointer}: string ${JSON.stringify(value)} failed pattern ${schema.pattern}`);
      }
    }
  }

  if (typeof value === 'number') {
    if (typeof schema.minimum === 'number' && value < schema.minimum) {
      errors.push(`${pointer}: number ${value} < minimum ${schema.minimum}`);
    }
    if (typeof schema.maximum === 'number' && value > schema.maximum) {
      errors.push(`${pointer}: number ${value} > maximum ${schema.maximum}`);
    }
  }

  if (Array.isArray(value) && schema.items !== undefined) {
    if (Array.isArray(schema.items)) {
      for (let i = 0; i < Math.min(value.length, schema.items.length); i += 1) {
        errors.push(...validateNode(rootSchema, schema.items[i], value[i], `${pointer}[${i}]`));
      }
    } else {
      for (let i = 0; i < value.length; i += 1) {
        errors.push(...validateNode(rootSchema, schema.items, value[i], `${pointer}[${i}]`));
      }
    }
  }

  if (value !== null && typeof value === 'object' && !Array.isArray(value)) {
    if (Array.isArray(schema.required)) {
      for (const reqKey of schema.required) {
        if (!(reqKey in value)) {
          errors.push(`${pointer}: missing required property '${reqKey}'`);
        }
      }
    }

    const props = schema.properties && typeof schema.properties === 'object' ? schema.properties : {};
    for (const [key, propVal] of Object.entries(value)) {
      const childPtr = `${pointer}.${key}`;
      if (key in props) {
        errors.push(...validateNode(rootSchema, props[key], propVal, childPtr));
      } else if (schema.additionalProperties === false) {
        errors.push(`${childPtr}: additional property '${key}' is not allowed`);
      } else if (
        schema.additionalProperties &&
        typeof schema.additionalProperties === 'object'
      ) {
        errors.push(...validateNode(rootSchema, schema.additionalProperties, propVal, childPtr));
      }
    }
  }

  return errors;
}

const loadedSchemas = new Map();
for (const fileName of REQUIRED_SCHEMAS) {
  const fullPath = path.join(schemasDir, fileName);
  if (!fs.existsSync(fullPath)) {
    console.error(`FAIL missing schema file: ${fullPath}`);
    process.exit(1);
  }
  const parsed = JSON.parse(fs.readFileSync(fullPath, 'utf8'));
  if (typeof parsed.$schema !== 'string' || typeof parsed.title !== 'string') {
    console.error(`FAIL invalid JSON Schema header in ${fileName}`);
    process.exit(1);
  }
  loadedSchemas.set(fileName, parsed);
}

function readJsonFixture(relPath) {
  const fullPath = path.join(repoRoot, relPath);
  if (!fs.existsSync(fullPath)) {
    console.error(`FAIL missing fixture file: ${fullPath}`);
    process.exit(1);
  }
  return JSON.parse(fs.readFileSync(fullPath, 'utf8'));
}

const VALID_FIXTURES = [
  {
    name: 'codelab.json (Go/GoFrame pack)',
    schema: 'codelab-pack.v1.schema.json',
    data: {
      schemaVersion: '1.0',
      courseId: 'go-goframe-practice',
      title: 'Go & GoFrame Practical Handbook',
      description: 'Exercises 0-9 from first principles',
      language: 'go',
      entryFile: 'README.md',
      starterDir: 'starter',
      checksFile: '.codelab-checks.json',
      dependencies: ['go>=1.22'],
      chapters: [{ chapterId: 'ex-1', title: 'Exercise 1: /healthz' }],
    },
  },
  {
    name: 'codelab.json (extended CourseChapterRef with kind/match/defaultCwd/entryFile)',
    schema: 'codelab-pack.v1.schema.json',
    data: {
      schemaVersion: '1.0',
      courseId: 'go-goframe-practice',
      title: 'Go & GoFrame Practical Handbook',
      description: 'Exercises 0-9 from first principles',
      language: 'go',
      entryFile: 'README.md',
      chapters: [
        {
          chapterId: 'ex-0',
          title: 'Overview',
          kind: 'reading',
          match: {
            anchor: 'overview',
            headingExact: 'Overview',
            headingPrefix: 'Over',
          },
          defaultCwd: null,
          entryFile: null,
        },
        {
          chapterId: 'ex-2',
          title: 'Exercise 2',
          kind: 'exercise',
          match: { headingPrefix: 'Exercise 2' },
          defaultCwd: 'task-api',
          entryFile: 'task-api/main.go',
        },
      ],
    },
  },
  {
    name: 'builtin-courses/go-goframe-practice/codelab.json',
    schema: 'codelab-pack.v1.schema.json',
    data: readJsonFixture('builtin-courses/go-goframe-practice/codelab.json'),
  },
  {
    name: 'builtin-courses/go-goframe-practice/.codelab-checks.json',
    schema: 'codelab-checks.v1.schema.json',
    data: readJsonFixture('builtin-courses/go-goframe-practice/.codelab-checks.json'),
  },
  {
    name: '.codelab-checks.json (PRD.md §4 M1 verbatim multi-chapter)',
    schema: 'codelab-checks.v1.schema.json',
    data: {
      schemaVersion: '1.0',
      courseId: 'go-goframe-practice',
      language: 'go',
      chapters: [
        {
          chapterId: 'ex-1',
          requiresDaemon: { label: 'go run .', port: 8080 },
          checks: [
            {
              id: 'v1',
              title: 'GET /healthz 返回 200 {"status":"ok"}',
              type: 'http_assert',
              request: { method: 'GET', url: 'http://127.0.0.1:8080/healthz' },
              expect: { status: 200, bodyContains: '"status":"ok"' },
            },
            {
              id: 'v2',
              title: 'go test -race ./... 零竞态通过',
              type: 'command_test',
              cwd: 'task-api',
              command: 'go test -json -race ./...',
              timeoutSec: 10,
              expectExitCode: 0,
              forbidOutput: ['WARNING: DATA RACE'],
            },
          ],
        },
      ],
    },
  },
  {
    name: '.codelab-checks.json (single-chapter author preview)',
    schema: 'codelab-checks.v1.schema.json',
    data: {
      schemaVersion: '1.0',
      chapterId: 'ex-1',
      language: 'ts',
      checks: [
        {
          id: 'f1',
          title: 'transform pipeline passes',
          type: 'function_test',
          language: 'ts',
          entryFile: 'src/pipeline.ts',
          testFile: 'src/pipeline.test.ts',
        },
        {
          id: 's1',
          title: 'verified edge cases',
          type: 'self_check',
        },
      ],
    },
  },
  {
    name: 'workspace.json (ARCHITECTURE.md §3.4.1 verbatim)',
    schema: 'workspace-meta.v1.schema.json',
    data: {
      schemaVersion: 1,
      workspaceId: 'ws_01J00000000000000000000000',
      workspaceRoot: '/Users/dev/go-practice',
      courseSource: {
        kind: 'builtin',
        courseId: 'go-goframe-practice',
        entryFile: 'README.md',
      },
      packFingerprint: 'sha256:abcdef',
      hydratedFiles: ['task-api/go.mod', 'task-api/main.go'],
      activeChapterId: 'ex-1-healthz',
      openTabs: ['task-api/main.go', 'task-api/main_test.go'],
      activeTab: 'task-api/main.go',
      layout: { manualWidthPct: 35, bottomHeightPx: 256, focusMode: false },
      updatedAt: '2026-11-22T12:00:00Z',
    },
  },
  {
    name: 'progress.json (ARCHITECTURE.md §3.4.1 verbatim)',
    schema: 'workspace-progress.v1.schema.json',
    data: {
      schemaVersion: 1,
      courseId: 'go-goframe-practice',
      chapters: {
        'ex-1-healthz': {
          status: 'completed',
          completedAt: '2026-11-22T12:10:00Z',
          lastRunId: 'run_01J00000000000000000000000',
          autoChecks: {
            v1: { passed: true, latencyMs: 4, checkedAt: '2026-11-22T12:10:00Z' },
            v2: { passed: true, latencyMs: 820, checkedAt: '2026-11-22T12:10:00Z' },
          },
          selfChecks: { s1: true },
        },
      },
    },
  },
  {
    name: 'metrics.json (valid closed 13-event record)',
    schema: 'workspace-metrics.v1.schema.json',
    data: {
      schemaVersion: 1,
      events: [
        {
          id: 'evt_01J00000000000000000000000',
          name: 'app_launched',
          ts: '2026-11-22T12:00:00Z',
          payload: { locale: 'zh-CN' },
        },
        {
          id: 'evt_01J00000000000000000000001',
          name: 'chapter_marked_done',
          ts: '2026-11-22T12:10:00Z',
          payload: { chapterIndex: 1, autoPassed: true },
        },
      ],
    },
  },
  {
    name: 'chat.json (valid chapter chat history)',
    schema: 'workspace-chat.v1.schema.json',
    data: {
      schemaVersion: 1,
      chapters: {
        'ex-1-healthz': {
          chapterId: 'ex-1-healthz',
          updatedAt: '2026-11-22T12:05:00Z',
          messages: [
            {
              id: 'msg_1',
              role: 'user',
              content: 'Why did /healthz return 404?',
              teachingMode: 'socratic',
              createdAt: '2026-11-22T12:04:00Z',
            },
          ],
        },
      },
    },
  },
];

const INVALID_FIXTURES = [
  {
    name: 'codelab-checks with numeric schemaVersion 2',
    schema: 'codelab-checks.v1.schema.json',
    data: {
      schemaVersion: 2,
      courseId: 'go-goframe-practice',
      language: 'go',
      chapters: [],
    },
  },
  {
    name: 'codelab-checks with unknown check type',
    schema: 'codelab-checks.v1.schema.json',
    data: {
      schemaVersion: '1.0',
      courseId: 'go-goframe-practice',
      language: 'go',
      chapters: [
        {
          chapterId: 'ex-1',
          checks: [{ id: 'x1', title: 'bad', type: 'arbitrary_shell' }],
        },
      ],
    },
  },
  {
    name: 'codelab-pack with wrong schemaVersion "2.0"',
    schema: 'codelab-pack.v1.schema.json',
    data: {
      schemaVersion: '2.0',
      courseId: 'demo',
      title: 'Demo',
      description: 'Desc',
      language: 'go',
      entryFile: 'README.md',
    },
  },
  {
    name: 'codelab-pack with invalid ChapterKind "quiz"',
    schema: 'codelab-pack.v1.schema.json',
    data: {
      schemaVersion: '1.0',
      courseId: 'demo',
      title: 'Demo',
      description: 'Desc',
      language: 'go',
      entryFile: 'README.md',
      chapters: [
        {
          chapterId: 'ex-1',
          title: 'Exercise 1',
          kind: 'quiz',
        },
      ],
    },
  },
  {
    name: 'workspace-meta with string schemaVersion "1" and missing workspaceId',
    schema: 'workspace-meta.v1.schema.json',
    data: {
      schemaVersion: '1',
      workspaceRoot: '/Users/dev/go-practice',
      courseSource: { kind: 'builtin', courseId: 'go-goframe-practice', entryFile: 'README.md' },
      packFingerprint: 'sha256:abcdef',
      layout: { manualWidthPct: 35, bottomHeightPx: 256, focusMode: false },
      updatedAt: '2026-11-22T12:00:00Z',
    },
  },
  {
    name: 'workspace-meta with traversal RelWorkspacePath "../etc/passwd"',
    schema: 'workspace-meta.v1.schema.json',
    data: {
      schemaVersion: 1,
      workspaceId: 'ws_1',
      workspaceRoot: '/Users/dev/go-practice',
      courseSource: { kind: 'builtin', courseId: 'go-goframe-practice', entryFile: 'README.md' },
      packFingerprint: 'sha256:abcdef',
      openTabs: ['../etc/passwd'],
      layout: { manualWidthPct: 35, bottomHeightPx: 256, focusMode: false },
      updatedAt: '2026-11-22T12:00:00Z',
    },
  },
  {
    name: 'workspace-progress with schemaVersion 2',
    schema: 'workspace-progress.v1.schema.json',
    data: {
      schemaVersion: 2,
      courseId: 'go-goframe-practice',
      chapters: {},
    },
  },
  {
    name: 'workspace-metrics with non-whitelisted event name',
    schema: 'workspace-metrics.v1.schema.json',
    data: {
      schemaVersion: 1,
      events: [
        {
          id: 'evt_bad',
          name: 'raw_source_uploaded',
          ts: '2026-11-22T12:00:00Z',
          payload: {},
        },
      ],
    },
  },
  {
    name: 'workspace-chat with invalid ChatRole',
    schema: 'workspace-chat.v1.schema.json',
    data: {
      schemaVersion: 1,
      chapters: {
        'ex-1': {
          chapterId: 'ex-1',
          updatedAt: '2026-11-22T12:05:00Z',
          messages: [
            {
              id: 'm1',
              role: 'moderator',
              content: 'invalid',
              teachingMode: null,
              createdAt: '2026-11-22T12:05:00Z',
            },
          ],
        },
      },
    },
  },
];

let failed = false;

console.log(`Loaded ${loadedSchemas.size} schemas from ${schemasDir}`);

for (const fixture of VALID_FIXTURES) {
  const schema = loadedSchemas.get(fixture.schema);
  const errors = validateNode(schema, schema, fixture.data, '$');
  if (errors.length === 0) {
    console.log(`PASS [valid]   ${fixture.name} (${fixture.schema})`);
  } else {
    failed = true;
    console.error(`FAIL [valid]   ${fixture.name} (${fixture.schema})`);
    for (const err of errors) {
      console.error(`  - ${err}`);
    }
  }
}

for (const fixture of INVALID_FIXTURES) {
  const schema = loadedSchemas.get(fixture.schema);
  const errors = validateNode(schema, schema, fixture.data, '$');
  if (errors.length > 0) {
    console.log(`PASS [reject]  ${fixture.name} -> ${errors[0]}`);
  } else {
    failed = true;
    console.error(`FAIL [reject]  ${fixture.name} unexpectedly passed ${fixture.schema}`);
  }
}

if (failed) {
  process.exit(1);
}
console.log('All schema fixture validations passed.');

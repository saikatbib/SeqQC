import { checkSchemas } from './check-schemas.mjs';
import { checkMetricAssignments } from './check-metric-assignments.mjs';
import { checkDependencyCandidates } from './check-dependency-candidates.mjs';
import { checkDesktopPolicy } from './check-desktop-policy.mjs';
import { checkProviderApprovals } from './check-provider-approvals.mjs';

const schemas = await checkSchemas();
const metrics = await checkMetricAssignments();
const dependencies = await checkDependencyCandidates();
const desktopPolicy = await checkDesktopPolicy();
await checkProviderApprovals();

process.stdout.write(`Repository checks passed: ${schemas.schemas} schemas, ${schemas.examples} valid examples, ${schemas.invalidExamples} invalid examples, ${metrics.metrics} assigned metrics, ${dependencies.candidates} dependency candidates, ${desktopPolicy.capabilities} desktop capability permissions.\n`);

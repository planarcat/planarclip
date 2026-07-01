import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { WORKSPACE_GIT_REPOS, getWorkspaceRepoByKey, resolveRepoDir } from './workspace-repos.mjs';

const workspaceRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');

/** @param {string} key */
function runAnalyzeOne(key) {
	const { gitnexusName } = getWorkspaceRepoByKey(key);
	const repoDir = resolveRepoDir(workspaceRoot, getWorkspaceRepoByKey(key));

	if (!fs.existsSync(repoDir)) {
		console.error(`Repo path not found: ${repoDir}`);
		return 1;
	}

	console.log(`\n========== gitnexus analyze: ${gitnexusName} ==========\n${repoDir}\n`);

	const bashCmd = 'export HF_ENDPOINT=https://hf-mirror.com && gitnexus analyze --embeddings';
	const result = spawnSync('wsl', ['--cd', repoDir, '-e', 'bash', '-lc', bashCmd], {
		stdio: 'inherit',
	});

	return result.status ?? 1;
}

const arg = process.argv[2];
const validKeys = WORKSPACE_GIT_REPOS.map((r) => r.key);

if (!arg || arg === '--all') {
	let exitCode = 0;
	for (const { key } of WORKSPACE_GIT_REPOS) {
		const code = runAnalyzeOne(key);
		if (code !== 0) {
			exitCode = code;
		}
	}
	process.exit(exitCode);
}

if (arg === '--help' || arg === '-h') {
	console.error(`Usage: node scripts/gitnexus-analyze-repo.mjs [--all|${validKeys.join('|')}]`);
	process.exit(0);
}

if (!validKeys.includes(arg)) {
	console.error(`Unknown repo: ${arg}. Use: ${validKeys.join(', ')}`);
	process.exit(1);
}

process.exit(runAnalyzeOne(arg));

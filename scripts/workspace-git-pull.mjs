import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import {
	getWorkspaceRepoByKey,
	getWorkspaceRepoByRel,
	normalizeRepoRel,
	resolveRepoDir,
} from './workspace-repos.mjs';

const workspaceRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const analyzeScript = path.join(workspaceRoot, 'scripts', 'gitnexus-analyze-repo.mjs');

/** @param {string} repoDir @param {string[]} args @param {boolean} [inherit] */
function git(repoDir, args, inherit = true) {
	return spawnSync('git', args, {
		cwd: repoDir,
		stdio: inherit ? 'inherit' : 'pipe',
		encoding: 'utf8',
	});
}

/** @param {string} repoDir @param {string[]} args */
function gitOut(repoDir, args) {
	const result = git(repoDir, args, false);
	return (result.stdout ?? '').trim();
}

/** @param {string} repoDir */
function isDirty(repoDir) {
	return gitOut(repoDir, ['status', '--porcelain']).length > 0;
}

/** @param {string} repoDir @param {string} branch */
function ensureOnBranch(repoDir, branch) {
	const current = gitOut(repoDir, ['branch', '--show-current']);
	if (current === branch) {
		return { status: 0 };
	}

	const checkout = git(repoDir, ['checkout', branch]);
	if (checkout.status === 0) {
		return checkout;
	}

	const remoteRef = `origin/${branch}`;
	if (git(repoDir, ['rev-parse', '--verify', remoteRef], false).status === 0) {
		return git(repoDir, ['checkout', '-B', branch, remoteRef]);
	}

	return git(repoDir, ['checkout', '-b', branch]);
}

/** @param {import('./workspace-repos.mjs').WorkspaceGitRepo} repo */
export function formatGitPullLabel(repo) {
	if (repo.devBranch) {
		return `${repo.rel} (on ${repo.devBranch}, pull origin ${repo.pullBranch})`;
	}
	return `${repo.rel} (merge origin/${repo.pullBranch} into current branch)`;
}

/** @param {string} repoDir @param {import('./workspace-repos.mjs').WorkspaceGitRepo} repo */
export function runGitPullForRepo(repoDir, repo) {
	const { pullBranch, devBranch } = repo;

	console.log(`\n> git pull — ${formatGitPullLabel(repo)}\n`);

	const fetch = git(repoDir, ['fetch', 'origin', pullBranch]);
	if (fetch.status !== 0) {
		return fetch;
	}

	let stashed = false;
	if (isDirty(repoDir)) {
		console.log('[workspace-git-pull] 检测到未提交修改，先 git stash -u …');
		const stash = git(repoDir, ['stash', 'push', '-u', '-m', 'planarclip-workspace-pull']);
		if (stash.status === 0) {
			stashed = true;
		} else {
			console.error('[workspace-git-pull] stash 失败，继续尝试 pull（merge/checkout 可能仍失败）');
		}
	}

	let sync;
	if (devBranch) {
		const onBranch = ensureOnBranch(repoDir, devBranch);
		if (onBranch.status !== 0) {
			if (stashed) {
				git(repoDir, ['stash', 'pop']);
			}
			return onBranch;
		}
		sync = git(repoDir, ['pull', 'origin', pullBranch]);
	} else {
		const current = gitOut(repoDir, ['branch', '--show-current']);
		if (current === pullBranch) {
			sync = git(repoDir, ['pull', 'origin', pullBranch]);
		} else if (current) {
			console.log(
				`[workspace-git-pull] 保持分支 ${current}，合并 origin/${pullBranch} …`,
			);
			sync = git(repoDir, ['merge', `origin/${pullBranch}`, '--no-edit']);
		} else {
			console.error('[workspace-git-pull] 无法识别当前分支（detached HEAD？），请先 checkout 到工作分支');
			sync = { status: 1 };
		}
	}

	if (sync.status !== 0) {
		if (stashed) {
			git(repoDir, ['stash', 'pop']);
		}
		return sync;
	}

	if (stashed) {
		console.log('[workspace-git-pull] git stash pop …');
		const pop = git(repoDir, ['stash', 'pop']);
		if (pop.status !== 0) {
			console.error('[workspace-git-pull] stash pop 冲突，请在本仓库手动 git stash pop');
			return pop;
		}
	}

	return sync;
}

/** @param {string} key @param {{ analyze?: boolean }} [opts] */
export function runGitPullByKey(key, opts = {}) {
	const repo = getWorkspaceRepoByKey(key);
	const repoDir = resolveRepoDir(workspaceRoot, repo);

	if (!fs.existsSync(path.join(repoDir, '.git'))) {
		console.error(`Not a git repo: ${repoDir}`);
		return { status: 1 };
	}

	const pull = runGitPullForRepo(repoDir, repo);
	if (pull.status !== 0) {
		return pull;
	}

	if (opts.analyze) {
		console.log(`\n> pnpm analyze (${repo.gitnexusName})\n`);
		const analyze = spawnSync(process.execPath, [analyzeScript, key], {
			cwd: workspaceRoot,
			stdio: 'inherit',
		});
		return { status: analyze.status ?? 1 };
	}

	return pull;
}

/** @param {string} repoDir @param {string} relPath */
export function runGitPullByRel(repoDir, relPath) {
	const normalized = normalizeRepoRel(relPath);
	const repo = getWorkspaceRepoByRel(normalized);

	if (!repo) {
		console.error(`[workspace-git-pull] 未在 workspace-repos 中配置: ${normalized}`);
		return { status: 1 };
	}

	return runGitPullForRepo(repoDir, repo);
}

const entry = process.argv[1] ? path.resolve(process.argv[1]) : '';
const isCli = entry === path.resolve(fileURLToPath(import.meta.url));

if (isCli) {
	const args = process.argv.slice(2).filter((a) => a !== '--');
	const analyze = args.includes('--analyze');
	const key = args.find((a) => !a.startsWith('-'));

	if (!key) {
		console.error('Usage: node scripts/workspace-git-pull.mjs <planarclip> [--analyze]');
		process.exit(1);
	}

	const result = runGitPullByKey(key, { analyze });
	process.exit(result.status ?? 1);
}

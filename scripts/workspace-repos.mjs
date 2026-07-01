import path from 'node:path';

/**
 * Git pull / GitNexus analyze 共用的工作区 Git 仓库列表
 *
 * @typedef {{ key: string; rel: string; gitnexusName: string; pullBranch: string; devBranch?: string }} WorkspaceGitRepo
 */

/** @type {WorkspaceGitRepo[]} */
export const WORKSPACE_GIT_REPOS = [
	{
		key: 'planarclip',
		rel: '.',
		gitnexusName: 'planarclip',
		pullBranch: 'master',
	},
];

/** @param {string} relPath */
export function normalizeRepoRel(relPath) {
	const normalized = relPath.replace(/\\/g, '/');
	if (normalized === '' || normalized === '.') {
		return '.';
	}
	return normalized;
}

/** @param {string} key */
export function getWorkspaceRepoByKey(key) {
	const repo = WORKSPACE_GIT_REPOS.find((r) => r.key === key);
	if (!repo) {
		throw new Error(
			`Unknown repo key: ${key}. Use: ${WORKSPACE_GIT_REPOS.map((r) => r.key).join(', ')}`,
		);
	}
	return repo;
}

/** @param {string} relPath normalized with / */
export function getWorkspaceRepoByRel(relPath) {
	const normalized = normalizeRepoRel(relPath);
	return WORKSPACE_GIT_REPOS.find((r) => r.rel === normalized) ?? null;
}

/** @param {string} workspaceRoot @param {WorkspaceGitRepo} repo */
export function resolveRepoDir(workspaceRoot, repo) {
	if (repo.rel === '.') {
		return workspaceRoot;
	}
	return path.join(workspaceRoot, ...repo.rel.split('/'));
}

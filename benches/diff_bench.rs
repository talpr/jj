use bencher::{benchmark_group, benchmark_main, Bencher};
use criterion_bencher_compat as bencher;
use jujutsu_lib::diff;

fn unchanged_lines(count: usize) -> (String, String) {
    let mut lines = vec![];
    for i in 0..count {
        lines.push(format!("left line {}\n", i));
    }
    (lines.join(""), lines.join(""))
}

fn modified_lines(count: usize) -> (String, String) {
    let mut left_lines = vec![];
    let mut right_lines = vec![];
    for i in 0..count {
        left_lines.push(format!("left line {}\n", i));
        right_lines.push(format!("right line {}\n", i));
    }
    (left_lines.join(""), right_lines.join(""))
}

fn reversed_lines(count: usize) -> (String, String) {
    let mut left_lines = vec![];
    for i in 0..count {
        left_lines.push(format!("left line {}\n", i));
    }
    let mut right_lines = left_lines.clone();
    right_lines.reverse();
    (left_lines.join(""), right_lines.join(""))
}

fn bench_diff_1k_unchanged_lines(b: &mut Bencher) {
    let (left, right) = unchanged_lines(1000);
    b.iter(|| diff::diff(left.as_bytes(), right.as_bytes()));
}

fn bench_diff_10k_unchanged_lines(b: &mut Bencher) {
    let (left, right) = unchanged_lines(10000);
    b.iter(|| diff::diff(left.as_bytes(), right.as_bytes()));
}

fn bench_diff_1k_modified_lines(b: &mut Bencher) {
    let (left, right) = modified_lines(1000);
    b.iter(|| diff::diff(left.as_bytes(), right.as_bytes()));
}

fn bench_diff_10k_modified_lines(b: &mut Bencher) {
    let (left, right) = modified_lines(10000);
    b.iter(|| diff::diff(left.as_bytes(), right.as_bytes()));
}

fn bench_diff_1k_lines_reversed(b: &mut Bencher) {
    let (left, right) = reversed_lines(1000);
    b.iter(|| diff::diff(left.as_bytes(), right.as_bytes()));
}

fn bench_diff_10k_lines_reversed(b: &mut Bencher) {
    let (left, right) = reversed_lines(10000);
    b.iter(|| diff::diff(left.as_bytes(), right.as_bytes()));
}

fn bench_diff_git_git_read_tree_c(b: &mut Bencher) {
    b.iter(|| {
        diff::diff(
            br##"/*
 * GIT - The information manager from hell
 *
 * Copyright (C) Linus Torvalds, 2005
 */
#include "#cache.h"

static int unpack(unsigned char *sha1)
{
	void *buffer;
	unsigned long size;
	char type[20];

	buffer = read_sha1_file(sha1, type, &size);
	if (!buffer)
		usage("unable to read sha1 file");
	if (strcmp(type, "tree"))
		usage("expected a 'tree' node");
	while (size) {
		int len = strlen(buffer)+1;
		unsigned char *sha1 = buffer + len;
		char *path = strchr(buffer, ' ')+1;
		unsigned int mode;
		if (size < len + 20 || sscanf(buffer, "%o", &mode) != 1)
			usage("corrupt 'tree' file");
		buffer = sha1 + 20;
		size -= len + 20;
		printf("%o %s (%s)\n", mode, path, sha1_to_hex(sha1));
	}
	return 0;
}

int main(int argc, char **argv)
{
	int fd;
	unsigned char sha1[20];

	if (argc != 2)
		usage("read-tree <key>");
	if (get_sha1_hex(argv[1], sha1) < 0)
		usage("read-tree <key>");
	sha1_file_directory = getenv(DB_ENVIRONMENT);
	if (!sha1_file_directory)
		sha1_file_directory = DEFAULT_DB_ENVIRONMENT;
	if (unpack(sha1) < 0)
		usage("unpack failed");
	return 0;
}
"##,
            br##"/*
 * GIT - The information manager from hell
 *
 * Copyright (C) Linus Torvalds, 2005
 */
#include "#cache.h"

static void create_directories(const char *path)
{
	int len = strlen(path);
	char *buf = malloc(len + 1);
	const char *slash = path;

	while ((slash = strchr(slash+1, '/')) != NULL) {
		len = slash - path;
		memcpy(buf, path, len);
		buf[len] = 0;
		mkdir(buf, 0700);
	}
}

static int create_file(const char *path)
{
	int fd = open(path, O_WRONLY | O_TRUNC | O_CREAT, 0600);
	if (fd < 0) {
		if (errno == ENOENT) {
			create_directories(path);
			fd = open(path, O_WRONLY | O_TRUNC | O_CREAT, 0600);
		}
	}
	return fd;
}

static int unpack(unsigned char *sha1)
{
	void *buffer;
	unsigned long size;
	char type[20];

	buffer = read_sha1_file(sha1, type, &size);
	if (!buffer)
		usage("unable to read sha1 file");
	if (strcmp(type, "tree"))
		usage("expected a 'tree' node");
	while (size) {
		int len = strlen(buffer)+1;
		unsigned char *sha1 = buffer + len;
		char *path = strchr(buffer, ' ')+1;
		char *data;
		unsigned long filesize;
		unsigned int mode;
		int fd;

		if (size < len + 20 || sscanf(buffer, "%o", &mode) != 1)
			usage("corrupt 'tree' file");
		buffer = sha1 + 20;
		size -= len + 20;
		data = read_sha1_file(sha1, type, &filesize);
		if (!data || strcmp(type, "blob"))
			usage("tree file refers to bad file data");
		fd = create_file(path);
		if (fd < 0)
			usage("unable to create file");
		if (write(fd, data, filesize) != filesize)
			usage("unable to write file");
		fchmod(fd, mode);
		close(fd);
		free(data);
	}
	return 0;
}

int main(int argc, char **argv)
{
	int fd;
	unsigned char sha1[20];

	if (argc != 2)
		usage("read-tree <key>");
	if (get_sha1_hex(argv[1], sha1) < 0)
		usage("read-tree <key>");
	sha1_file_directory = getenv(DB_ENVIRONMENT);
	if (!sha1_file_directory)
		sha1_file_directory = DEFAULT_DB_ENVIRONMENT;
	if (unpack(sha1) < 0)
		usage("unpack failed");
	return 0;
}
"##,
        )
    });
}

benchmark_group!(
    benches,
    bench_diff_1k_unchanged_lines,
    bench_diff_10k_unchanged_lines,
    bench_diff_1k_modified_lines,
    bench_diff_10k_modified_lines,
    bench_diff_1k_lines_reversed,
    bench_diff_10k_lines_reversed,
    bench_diff_git_git_read_tree_c,
);
benchmark_main!(benches);

#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>

int main(int argc, char *argv[])
{
	int sleep_secs;
	if (argc != 2 || ( (sleep_secs = atoi(argv[1])) ) <= 0 ) {
		fprintf(stderr, "Usage: %s SECS\n", argv[0]);
		return EXIT_FAILURE;
	}

	sleep(sleep_secs);


	return EXIT_SUCCESS;
}


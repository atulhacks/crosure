/* Benign crackme used as a test fixture. Not malware. */
#include <stdio.h>
#include <string.h>

static const char *beacon = "http://c2.example.invalid/beacon";
static unsigned char secret[] = {0x31, 0x26, 0x30, 0x26, 0x31, 0x30, 0x26, 0x00};

void decode(unsigned char *buf, size_t n, unsigned char key) {
    for (size_t i = 0; i < n; i++) buf[i] ^= key;
}

int check_password(const char *input) {
    unsigned char tmp[sizeof(secret)];
    memcpy(tmp, secret, sizeof(secret));
    decode(tmp, sizeof(secret) - 1, 0x43);
    return strcmp(input, (const char *)tmp) == 0;
}

int main(int argc, char **argv) {
    if (argc < 2) {
        printf("Usage: %s <password>\n", argv[0]);
        return 1;
    }
    if (check_password(argv[1])) {
        puts("Correct! Reporting to beacon...");
        puts(beacon);
        return 0;
    }
    puts("Wrong password");
    return 2;
}

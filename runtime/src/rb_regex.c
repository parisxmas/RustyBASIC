#include "rb_runtime.h"
#include <stdio.h>
#include <string.h>
#include <regex.h>

int32_t rb_regex_match(rb_string_t* pattern, rb_string_t* text) {
    char pat[256], txt[4096];
    snprintf(pat, sizeof(pat), "%.*s", pattern->len, pattern->data);
    snprintf(txt, sizeof(txt), "%.*s", text->len, text->data);
    regex_t re;
    if (regcomp(&re, pat, REG_EXTENDED | REG_NOSUB) != 0) return 0;
    int result = (regexec(&re, txt, 0, NULL, 0) == 0) ? 1 : 0;
    regfree(&re);
    return result;
}

rb_string_t* rb_regex_find(rb_string_t* pattern, rb_string_t* text) {
    char pat[256], txt[4096];
    snprintf(pat, sizeof(pat), "%.*s", pattern->len, pattern->data);
    snprintf(txt, sizeof(txt), "%.*s", text->len, text->data);
    regex_t re;
    regmatch_t match;
    if (regcomp(&re, pat, REG_EXTENDED) != 0) return rb_string_from_cstr("");
    if (regexec(&re, txt, 1, &match, 0) == 0) {
        int len = match.rm_eo - match.rm_so;
        char buf[4096];
        snprintf(buf, sizeof(buf), "%.*s", len, txt + match.rm_so);
        regfree(&re);
        return rb_string_from_cstr(buf);
    }
    regfree(&re);
    return rb_string_from_cstr("");
}

rb_string_t* rb_regex_replace(rb_string_t* pattern, rb_string_t* text, rb_string_t* replacement) {
    char pat[256], txt[4096], rep[4096];
    snprintf(pat, sizeof(pat), "%.*s", pattern->len, pattern->data);
    snprintf(txt, sizeof(txt), "%.*s", text->len, text->data);
    snprintf(rep, sizeof(rep), "%.*s", replacement->len, replacement->data);
    regex_t re;
    regmatch_t match;
    if (regcomp(&re, pat, REG_EXTENDED) != 0) return rb_string_from_cstr(txt);
    char result[8192] = "";
    char *pos = txt;
    while (regexec(&re, pos, 1, &match, 0) == 0) {
        strncat(result, pos, match.rm_so);
        strcat(result, rep);
        pos += match.rm_eo;
    }
    strcat(result, pos);
    regfree(&re);
    return rb_string_from_cstr(result);
}

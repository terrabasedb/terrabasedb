#!/usr/bin/perl -w
=pod
All credits for the random unicode string generation logic go to Paul Sarena who released
the original version here: https://github.com/bits/UTF-8-Unicode-Test-Documents and released
it under the BSD 3-Clause "New" or "Revised" License 
=cut
use strict;
use warnings qw( FATAL utf8 );
use utf8;  # tell Perl parser there are non-ASCII characters in this lexical scope
use open qw( :encoding(UTF-8) :std );  # Declare that anything that opens a filehandles within this lexical scope is to assume that that stream is encoded in UTF-8 unless you tell it otherwise

use Encode;
use HTML::Entities;

my $html_pre = q|<?xml version="1.0" encoding="utf-8"?>
<!DOCTYPE html PUBLIC "-//W3C//DTD XHTML 1.0 Transitional//EN" "http://www.w3.org/TR/xhtml1/DTD/xhtml1-transitional.dtd">
<html xmlns="http://www.w3.org/1999/xhtml" xml:lang="en" lang="en">
<head>
	<title>UTF-8 Codepoint Sequence</title>
</head>
<body>|;

my $html_post = q|</body>
</html>|;

my $output_directory = './utf8/';

my $utf8_seq;

#    0000–​FFFF Plane 0:      Basic Multilingual Plane
#  10000–​1FFFF Plane 1:      Supplementary Multilingual Plane
#  20000–​2FFFF Plane 2:      Supplementary Ideographic Plane
#  30000–​DFFFF Planes 3–13:  Unassigned
#  E0000–​EFFFF Plane 14:     Supplement­ary Special-purpose Plane
# F0000–​10FFFF Planes 15–16: Supplement­ary Private Use Area

foreach my $separator ('', ' ') {
	foreach my $end (0xFF, 0xFFF, 0xFFFF, 0x1FFFF, 0x2FFFF, 0x10FFFF) {

		# UTF-8 codepoint sequence of assigned, printable codepoints
		$utf8_seq = gen_seq({
			start => 0x00,
			end => $end,
			separator => $separator,
			skip_unprintable => 1,
				replace_unprintable => 1,
			skip_unassigned => 1,
			writefiles => ($separator ? 'txt,html' : 'txt')
		});


		# UTF-8 codepoint sequence of assigned, printable and unprintable codepoints as-is
		$utf8_seq = gen_seq({
			start => 0x00,
			end => $end,
			separator => $separator,
			skip_unprintable => 0,
				replace_unprintable => 0,
			skip_unassigned => 1,
			writefiles => ($separator ? 'txt,html' : 'txt')
		});
		# UTF-8 codepoint sequence of assigned, printable and unprintable codepoints replaced
		$utf8_seq = gen_seq({
			start => 0x00,
			end => $end,
			separator => $separator,
			skip_unprintable => 0,
				replace_unprintable => 1,
			skip_unassigned => 1,
			writefiles => ($separator ? 'txt,html' : 'txt')
		});


		# UTF-8 codepoint sequence of assinged and unassigned, printable and unprintable codepoints as-is
		$utf8_seq = gen_seq({
			start => 0x00,
			end => $end,
			separator => $separator,
			skip_unprintable => 0,
				replace_unprintable => 0,
			skip_unassigned => 0,
			writefiles => ($separator ? 'txt,html' : 'txt')
		});
		# UTF-8 codepoint sequence of assinged and unassigned, printable and unprintable codepoints replaced
		$utf8_seq = gen_seq({
			start => 0x00,
			end => $end,
			separator => $separator,
			skip_unprintable => 0,
				replace_unprintable => 1,
			skip_unassigned => 0,
			writefiles => ($separator ? 'txt,html' : 'txt')
		});

	}
}

# print Encode::encode('UTF-8', $utf8_seq), "\n";



sub gen_seq{
	my $config = shift;

	$config->{start}               = 0x00        unless defined $config->{start};
	$config->{end}                 = 0x10FFFF    unless defined $config->{end};
	$config->{skip_unassigned}     = 1           unless defined $config->{skip_unassigned};
	$config->{skip_unprintable}    = 1           unless defined $config->{skip_unprintable};
	$config->{replace_unprintable} = 1           unless defined $config->{replace_unprintable};
	$config->{separator}           = ' '         unless defined $config->{separator};
	$config->{newlines_every}      = 50          unless defined $config->{newlines_every};
	$config->{writefiles}          = 'text,html' unless defined $config->{writefiles};

	my $utf8_seq;
	my $codepoints_this_line = 0;
	my $codepoints_printed = 0;

	for my $i ($config->{start} .. $config->{end}) {

		next if ($i >= 0xD800 && $i <= 0xDFFF); # high and low surrogate halves used by UTF-16 (U+D800 through U+DFFF) are not legal Unicode values, and the UTF-8 encoding of them is an invalid byte sequence
		next if ($i >= 0xFDD0 && $i <= 0xFDEF); # Non-characters
		next if ( # Non-characters
			$i ==   0xFFFE || $i ==   0xFFFF ||
			$i ==  0x1FFFE || $i ==  0x1FFFF ||
			$i ==  0x2FFFE || $i ==  0x2FFFF ||
			$i ==  0x3FFFE || $i ==  0x3FFFF ||
			$i ==  0x4FFFE || $i ==  0x4FFFF ||
			$i ==  0x5FFFE || $i ==  0x5FFFF ||
			$i ==  0x6FFFE || $i ==  0x6FFFF ||
			$i ==  0x7FFFE || $i ==  0x7FFFF ||
			$i ==  0x8FFFE || $i ==  0x8FFFF ||
			$i ==  0x9FFFE || $i ==  0x9FFFF ||
			$i ==  0xaFFFE || $i ==  0xAFFFF ||
			$i ==  0xbFFFE || $i ==  0xBFFFF ||
			$i ==  0xcFFFE || $i ==  0xCFFFF ||
			$i ==  0xdFFFE || $i ==  0xDFFFF ||
			$i ==  0xeFFFE || $i ==  0xEFFFF ||
			$i ==  0xfFFFE || $i ==  0xFFFFF ||
			$i == 0x10FFFE || $i == 0x10FFFF
		);

		my $codepoint = chr($i);

		# skip unassiggned codepoints
		next if $config->{skip_unassigned} && $codepoint !~ /^\p{Assigned}/o;

		if ( $codepoint =~ /^\p{IsPrint}/o ) {
			$utf8_seq .= $codepoint;
		} else { # not printable
			next if $config->{skip_unprintable};
			# include unprintable or replace it
			$utf8_seq .= $config->{replace_unprintable} ? '�' : $codepoint;
		}

		$codepoints_printed++;

		if ($config->{separator}) {
			if ($config->{newlines_every} && $codepoints_this_line++ == $config->{newlines_every}) {
				$utf8_seq .= "\n";
				$codepoints_this_line = 0;
			} else {
				$utf8_seq .= $config->{separator};
			}
		}
	}

	utf8::upgrade($utf8_seq);


	if ($config->{writefiles}) {

		my $filebasename = 'utf8_sequence_' .
			(sprintf '%#x', $config->{start}) .
			'-' .
			(sprintf '%#x', $config->{end}) .
			($config->{skip_unassigned} ? '_assigned' : '_including-unassigned') .
			($config->{skip_unprintable} ? '_printable' : '_including-unprintable') .
			(!$config->{skip_unprintable} ?
				($config->{replace_unprintable} ? '-replaced' : '-asis') :
				''
			) .
			($config->{separator} ?
				($config->{newlines_every} ? '' : '_without-newlines') :
				'_unseparated'
			);


		my $title = 'UTF-8 codepoint sequence' .
			($config->{skip_unassigned} ? ' of assigned' : ' of assinged and unassigned') .
			($config->{skip_unprintable} ? ', printable' : ', with unprintable') .
			(!$config->{skip_unprintable} ?
				($config->{replace_unprintable} ? ' codepoints replaced' : ' codepoints as-is') :
				' codepoints'
			) .
			' in the range ' .
			(sprintf '%#x', $config->{start}) .
			'-' .
			(sprintf '%#x', $config->{end}) .
			($config->{newlines_every} ? '' : ', as a long string without newlines');

		my $html_pre_custom = $html_pre;
		$html_pre_custom =~ s|UTF\-8 codepoint sequence|$title|;


		my $filename = ${output_directory} . ($config->{separator} ? '' : 'un') . 'separated/' . ${filebasename};

		if ($config->{writefiles} =~ /te?xt/) {
			open FH, ">${filename}.txt" or die "cannot open $filename: $!";
			print FH $utf8_seq;
			close FH;
		}

		if ($config->{writefiles} =~ /html/) {
			open FH, ">${filename}_unescaped.html" or die "cannot open $filename: $!";
			print FH $html_pre_custom, $utf8_seq, $html_post;
			close FH;
		}

		# open FH, ">${output_directory}${filebasename}_escaped.html";
		# print FH $html_pre_custom, HTML::Entities::encode_entities($utf8_seq), $html_post;
		# close FH;

		print "Output $title ($codepoints_printed codepoints)\n";
	}

	return $utf8_seq;
}
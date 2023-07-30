ifeq ($(shell uname -s),Darwin)
	SEDOPT=-E
else
	SEDOPT=-r
endif

TROPT=LC_CTYPE=C tr

INKSCAPE=/Applications/Inkscape.app/Contents/Resources/bin/inkscape

MAINTEXFILES=$(shell grep -l '\\documentclass' $(wildcard *.tex))
MAINPDFFILES=$(patsubst %.tex,%.pdf,$(MAINTEXFILES))

default: $(MAINPDFFILES)

mrproper:
	rm -rf auto
	rm -f `find . -name \*~`





#### Generic functions

FileExists=$(shell if test -e $(1) ; then echo $(1) ; fi)

DirectRule=$(if $(call FileExists,$(1)),$(1))

IntermediateRule=$(if $(call FileExists,$(1)),$(2)$(eval TOCLEAN+=$(2)))



#### Functions related to includegraphics commands

FindMissingGraphicsExtension=$(or $(call IntermediateRule,$(1).tiff,$(1).ps,$(2)),$(call IntermediateRule,$(1).jpg,$(1).ps,$(2)),$(call IntermediateRule,$(1).pdf,$(1).ps,$(2)),$(call IntermediateRule,$(1).asy,$(1).eps,$(2)),$(call IntermediateRule,$(1).svg,$(1).eps,$(2)),$(call DirectRule,$(1),$(2)),$(call DirectRule,$(1).eps,$(2)),$(call DirectRule,$(1).ps,$(2)))

GraphicsDeps=$(foreach Dep,$(shell cat $(1) | sed $(SEDOPT) 's/^([^%]*)[%].*$$/\1/g' | sed $(SEDOPT) 's/\\(re)?newcommand[{][^}]*[}](\[[^\]]*\])?[{]([^{}]*|[{][^}]*[}])*[}]//g' | $(TROPT) -d '\n' | egrep -h -o '\\includegraphics(\[[^]]*\])?[{][^}]*[}]' | sed $(SEDOPT) 's/^[^{]*[{]([^}]*)[}]$$/\1 /g'),$(call FindMissingGraphicsExtension,$(Dep),$(1)))




#### Functions related to input and include commands

FindMissingTexExtension=$(or $(call IntermediateRule,$(1).fig,$(1).eps $(1).tex,$(2)), $(call DirectRule,$(1),$(2)),$(call DirectRule,$(1).tex,$(2)))

TexDeps=$(foreach Dep,$(shell cat $(1) | sed $(SEDOPT) 's/^([^%]*)[%].*$$/\1/g' | sed $(SEDOPT) 's/\\(re)?newcommand[{][^}]*[}](\[[^]]*\])?[{]([^{}]*|[{][^}]*[}])*[}]//g' | $(TROPT) -d '\n' | egrep -h -o '((\\input)|(\\include))[{][^}]*[}]' | sed $(SEDOPT) 's/^[^{]*[{]([^}]*)[}]$$/\1 /g'),$(call FindMissingTexExtension,$(Dep),$(1)))


#### Functions related to verbatiminput commands

VerbDeps=$(foreach Dep,$(shell cat $(1) | sed $(SEDOPT) 's/^([^%]*)[%].*$$/\1/g' | sed $(SEDOPT) 's/\\(re)?newcommand[{][^}]*[}](\[[^]]*\])?[{]([^{}]*|[{][^}]*[}])*}//g' | $(TROPT) -d '\n' | egrep -h -o '(\\verbatiminput)[{][^}]*[}]' | sed $(SEDOPT) 's/^[^{]*[{]([^}]*)[}]$$/\1 /g'),$(call DirectRule,$(Dep),$(1)))




#### Functions related to bibliography command

FindMissingBibExtension=$(or $(call DirectRule,$(1),$(2)),$(call DirectRule,$(1).bib,$(2)))

BibDeps=$(foreach Dep,$(shell cat $(1) | sed $(SEDOPT) 's/^([^%]*)[%].*$$/\1/g' | sed $(SEDOPT) 's/\\(re)?newcommand[{][^}]*[}](\[[^]]*\])?[{]([^{}]*|[{][^}]*[}])*[}]//g' | $(TROPT) -d '\n' | egrep -h -o '\\bibliography[{][^}]*[}]' | sed $(SEDOPT) 's/^[^{]*[{]([^}]*)[}]$$/\1 /g' | $(TROPT) ',' ' '),$(call FindMissingBibExtension,$(Dep),$(1)))




#### Recursive function that enumerate dependencies of a Tex file

AllDeps=$(foreach Dep,$(call TexDeps,$(1)),$(Dep) $(if $(call FileExists,$(Dep)),$(call AllDeps,$(Dep)))) $(call GraphicsDeps,$(1)) $(call VerbDeps,$(1)) $(call BibDeps,$(1))




#### User function that gathers dependencies of a Tex file and stores them in a specific variable, and stores in another variable intermediate files that have been generated and which can be removed safely


define TexMaker

TOCLEAN=
$(1)_DEPS=$(call AllDeps,$(1))
$(1)_TOCLEAN:=$(TOCLEAN)

$(patsubst %.tex,%.pdf,$(1)): $(1) $$($(1)_DEPS)

$(patsubst %.tex,%.ps,$(1)): $(1) $$($(1)_DEPS)

$(patsubst %.tex,%.dvi,$(1)): $(1) $$($(1)_DEPS)

clean_$(1):
#	rubber --clean --quiet $(1)
	rm -f $(patsubst %.tex,%.dvi,$(1)) $(patsubst %.tex,%.out,$(1)) $(patsubst %.tex,%.log,$(1)) $(patsubst %.tex,%.aux,$(1)) $(patsubst %.tex,%.vrb,$(1)) head.tmp body.tmp

clean: clean_$(1)

mrproper_$(1): clean_$(1)
	rm -f $(patsubst %.tex,%.ps,$(1)) $(patsubst %.tex,%.pdf,$(1)) $$($(1)_TOCLEAN)

mrproper: mrproper_$(1)



$(patsubst %.tex,%.dvi,$(1)): $(1)
#	rubber $$<
	latex $(1)
	latex $(1)
	latex $(1)

$(patsubst %.tex,%.ps,$(1)): $(patsubst %.tex,%.dvi,$(1))
	dvips -G -K -j -Pdownload35 -Z -o $$@ $$<


$(patsubst %.tex,%.pdf,$(1)): $(patsubst %.tex,%.dvi,$(1))
	dvips -z -G -K -j -Ppdf -Z -f $$< | gs -dNOPAUSE -dBATCH -sDEVICE=pdfwrite -dSubsetFonts=true -dEmbedAllFonts=true -dOptimize=true -dCompatibilityLevel=1.4 -dNOPLATFONTS -dDOPDFMARKS -dSAFER -dSTRICT -dPDFSETTINGS=/prepress -dConvertCMYKImagesToRGB=false -dProcessColorModel=/DeviceCMYK -sOutputFile=$$@ -


endef





#### Generic rules to build missing included files that can be automatically generated from source files

mrproper: clean

clean:



.SUFFIXES: .eps .ps .tex .jpg .tiff .asy .fig .svg .dvi .pdf





$(foreach MainTexFile,$(MAINTEXFILES),$(eval $(call TexMaker,$(MainTexFile))))







%.tex: %.fig %.eps
	fig2dev -L pstex_t -p $*.eps $< $*.tex

%.eps: %.fig
	fig2dev -L pstex $< $*.eps 


%.pdf: %.tiff
	convert $< $@

%.pdf: %.jpg
	pdfimage -o $@ $<

%.eps: %.pdf
	pdf2ps -sDEVICE=epswrite $< $@

%.eps: %.asy
	asy -cd $(dir $<) -o $(shell basename $@) -f eps $(shell basename $<)

%.eps: %.svg
#	./tools/fonts_in_svg.sh $<
	$(INKSCAPE) -z -T   -f $< -E $@




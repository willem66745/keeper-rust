// Show loader and disable the rest of the user interface
function show_loader() {
    $("#foo").addClass('ui-disabled');
    $.mobile.loading("show", {
        text: "loading",
        textVisible: true
    });
}

// Hide loader and activate the rest of the user interface
function hide_loader() {
    $.mobile.loading("hide");
    $("#foo").removeClass('ui-disabled');
}

// Retrieve all switch configurations and update web user interface
function load_switches() {
    show_loader();

    $.post("/api/switches", function(data) {
        var items = data.length;

        $("#content").empty();
        $("#content").hide();

        $.each(data, function(index, object) {
            var li = $("#content").append('<li class="ui-field-contain"><a href="#details">'
                    + '<h2>' + object + '</h2>'
                    + '<p><select name="flip_' + object + '" id="flip_' + object + '" data-role="slider" data-mini="true">'
                        + '<option value="off">Off</option>'
                        + '<option value="on">On</option>'
                    + '</select></p>'
                    + '</a></li>');

            li = li.find("li").last();
            var flip = li.find("#flip_" + object);
            flip.slider().change(function(event) {
                li.addClass('ui-disabled');
                $.post("/api/switch/" + object + "/" + $(this).val(), function(data) {
                    flip.val(data ? "on" : "off").slider("refresh");
                }).always(function() {
                    li.removeClass('ui-disabled');
                });
            });

            $.ajax({
                url: "/api/get/" + object,
                type: "POST",
                async: false,
                timeout: 2000
            }).done(function(data) {
                if (data.switch == true) {
                    // apply light theme when switch is enabled
                    //li.attr("data-theme", "a");
                    flip.val("on").slider("refresh");
                } else {
                    // apply dark theme when switch is enabled
                    //li.attr("data-theme", "b");
                    flip.val("off").slider("refresh");
                }
            }).fail(function() {
            }).always(function() {
                items--;
                if (items == 0) {
                    hide_loader();
                }
            });
        });

        $("#content").listview("refresh");
        $("#content").show();
    });
}

// Load web page and set event handlers
$(function() {
    load_switches();

    $("#refresh").click(function(event) {
        event.preventDefault();
        load_switches();
    });
});
